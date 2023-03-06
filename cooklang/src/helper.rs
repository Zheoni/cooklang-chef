use crate::{model::Ingredient, Recipe};

pub struct WithRecipe<'a, T, D> {
    _recipe: &'a Recipe<'a, D>,
    _val: &'a T,
}

impl<'a, D> WithRecipe<'a, Ingredient<'a>, D> {}
