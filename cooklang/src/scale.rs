//! Support for recipe scaling

use serde::Serialize;
use thiserror::Error;

use crate::{
    convert::Converter,
    quantity::{QuantityValue, TextValueError, Value},
    Recipe, ScaledRecipe,
};

/// Configures the scaling
#[derive(Debug, Clone, Copy)]
pub struct ScaleTarget {
    base: u32,
    target: u32,
    index: Option<usize>,
}

impl ScaleTarget {
    /// Creates a new [ScaleTarget].
    /// - `base` is the number of servings the recipe was initially written for.
    /// - `target` is the wanted number of servings.
    /// - `declared_servigs` is a slice with all the servings of the recipe metadata.
    ///
    /// Invalid parameters don't error here, but may do so in the
    /// scaling process.
    pub fn new(base: u32, target: u32, declared_servings: &[u32]) -> Self {
        ScaleTarget {
            base,
            target,
            index: declared_servings.iter().position(|&s| s == target),
        }
    }

    /// Get the scaling factor calculated
    pub fn factor(&self) -> f64 {
        self.target as f64 / self.base as f64
    }

    /// Get the index into a [ScalableValue::ByServings]
    pub fn index(&self) -> Option<usize> {
        self.index
    }

    /// Get the target servings
    pub fn target_servings(&self) -> u32 {
        self.target
    }
}

/// Possible scaled states of a recipe
#[derive(Debug)]
pub enum Scaled {
    /// The recipe was scaled to its based servings
    ///
    /// This is the values without scaling or if there are many values
    /// for a component, the first one.
    DefaultScaling,
    /// Scaled to a custom target
    Scaled(ScaledData),
}

/// Data from scaling a recipe
#[derive(Debug)]
pub struct ScaledData {
    /// What the target was
    pub target: ScaleTarget,
    /// The
    pub ingredients: Vec<ScaleOutcome>,
    pub cookware: Vec<ScaleOutcome>,
    pub timers: Vec<ScaleOutcome>,
}

/// Possible outcomes from scaling a component
#[derive(Debug, Clone, Serialize)]
pub enum ScaleOutcome {
    /// Success
    Scaled,
    /// Not changed becuse it doen't have to be changed
    Fixed,
    /// It has no quantity, so it can't be scaled
    NoQuantity,
    /// Error scaling
    Error(#[serde(skip_serializing)] ScaleError),
}

/// Possible errors during scaling process
#[derive(Debug, Error, Clone)]
pub enum ScaleError {
    #[error(transparent)]
    TextValueError(#[from] TextValueError),

    #[error("Value not scalable: {reason}")]
    NotScalable {
        value: QuantityValue,
        reason: &'static str,
    },

    #[error("Value scaling not defined for target servings")]
    NotDefined {
        target: ScaleTarget,
        value: QuantityValue,
    },
}

impl Recipe {
    /// Scale a recipe.
    ///
    /// Note that this returns a [ScaledRecipe] wich doesn't implement this
    /// method. A recipe can only be scaled once.
    pub fn scale(mut self, target: u32, converter: &Converter) -> ScaledRecipe {
        let target = if let Some(servings) = self.metadata.servings.as_ref() {
            let base = servings.first().copied().unwrap_or(1);
            ScaleTarget::new(base, target, servings)
        } else {
            ScaleTarget::new(1, target, &[])
        };

        if target.index() == Some(0) {
            return self.default_scale();
        }
        let ingredients = scale_many(target, &mut self.ingredients, |igr| {
            igr.quantity.as_mut().map(|q| &mut q.value)
        });
        self.ingredients.iter_mut().for_each(|i| {
            if let Some(q) = &mut i.quantity {
                q.fit(converter);
            }
        });
        let cookware = scale_many(target, &mut self.cookware, |ck| ck.quantity.as_mut());
        let timers = scale_many(target, &mut self.timers, |tm| Some(&mut tm.quantity.value));

        let data = ScaledData {
            target,
            ingredients,
            cookware,
            timers,
        };

        ScaledRecipe {
            name: self.name,
            metadata: self.metadata,
            sections: self.sections,
            ingredients: self.ingredients,
            cookware: self.cookware,
            timers: self.timers,
            inline_quantities: self.inline_quantities,
            data: Scaled::Scaled(data),
        }
    }

    /// Scale the recipe to the default values.
    ///
    /// This collapses the [ScalableValue::ByServings] to a single value.
    pub fn default_scale(mut self) -> ScaledRecipe {
        default_scale_many(&mut self.ingredients, |igr| {
            igr.quantity.as_mut().map(|q| &mut q.value)
        });
        default_scale_many(&mut self.cookware, |ck| ck.quantity.as_mut());
        default_scale_many(&mut self.timers, |tm| Some(&mut tm.quantity.value));

        ScaledRecipe {
            name: self.name,
            metadata: self.metadata,
            sections: self.sections,
            ingredients: self.ingredients,
            cookware: self.cookware,
            timers: self.timers,
            inline_quantities: self.inline_quantities,
            data: Scaled::DefaultScaling,
        }
    }
}

impl ScaledRecipe {
    /// Get the [ScaledData] from a recipe after scaling.
    pub fn scaled_data(&self) -> Option<&ScaledData> {
        if let Scaled::Scaled(data) = &self.data {
            Some(data)
        } else {
            None
        }
    }

    /// Shorthand to check if [Self::scaled_data] is [Scaled::DefaultScaling].
    pub fn is_default_scaled(&self) -> bool {
        matches!(self.data, Scaled::DefaultScaling)
    }
}

fn scale_many<'a, T: 'a>(
    target: ScaleTarget,
    components: &mut [T],
    extract: impl Fn(&mut T) -> Option<&mut QuantityValue>,
) -> Vec<ScaleOutcome> {
    let mut outcomes = Vec::with_capacity(components.len());
    for c in components {
        if let Some(value) = extract(c) {
            match value.clone().scale(target) {
                // ? Unnecesary clone maybe
                Ok((v, o)) => {
                    *value = v;
                    outcomes.push(o);
                }
                Err(e) => outcomes.push(ScaleOutcome::Error(e)),
            }
        } else {
            outcomes.push(ScaleOutcome::NoQuantity);
        }
    }
    outcomes
}

fn default_scale_many<'a, T: 'a>(
    components: &mut [T],
    extract: impl Fn(&mut T) -> Option<&mut QuantityValue>,
) {
    for c in components {
        if let Some(value) = extract(c) {
            *value = value.clone().default_scale();
        }
    }
}

impl QuantityValue {
    fn scale(self, target: ScaleTarget) -> Result<(QuantityValue, ScaleOutcome), ScaleError> {
        let (value, outcome) = match self {
            Self::Fixed(v) => (v, ScaleOutcome::Fixed),
            Self::Linear(v) => (v.scale(target.factor())?, ScaleOutcome::Scaled),
            Self::ByServings(ref v) => {
                if let Some(index) = target.index {
                    let Some(value) = v.get(index) else {
                        return Err(ScaleError::NotDefined { target, value: self });
                    };
                    (value.clone(), ScaleOutcome::Scaled)
                } else {
                    return Err(ScaleError::NotScalable {
                        value: self,
                        reason: "tried to scale a value linearly when it has the scaling defined",
                    });
                }
            }
        };
        Ok((Self::Fixed(value), outcome))
    }

    fn default_scale(self) -> Self {
        match self {
            v @ Self::Fixed(_) => v,
            Self::Linear(v) => Self::Fixed(v),
            Self::ByServings(v) => Self::Fixed(
                v.first()
                    .expect("scalable value servings list empty")
                    .clone(),
            ),
        }
    }
}

impl Value {
    fn scale(&self, factor: f64) -> Result<Value, ScaleError> {
        match self.clone() {
            Value::Number(n) => Ok(Value::Number(n * factor)),
            Value::Range(r) => Ok(Value::Range(r.start() * factor..=r.end() * factor)),
            v @ Value::Text(_) => Err(TextValueError(v).into()),
        }
    }
}
