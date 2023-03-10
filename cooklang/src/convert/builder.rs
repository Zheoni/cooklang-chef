use std::{collections::HashSet, sync::Arc};

use enum_map::{enum_map, EnumMap};
use thiserror::Error;

use super::{
    convert_f64,
    units_file::{BestUnits, Extend, Precedence, SIPrefix, UnitEntry, Units, UnitsFile, SI},
    BestConversions, BestConversionsStore, Converter, PhysicalQuantity, System, Unit, UnitIndex,
    UnknownUnit,
};

#[derive(Default)]
pub struct ConverterBuilder {
    all_units: Vec<Unit>,
    unit_index: UnitIndex,
    extend: Vec<Extend>,
    si: SI,
    best_units: EnumMap<PhysicalQuantity, Option<BestUnits>>,
    default_system: System,
}

impl ConverterBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_units_file(mut self, units: UnitsFile) -> Result<Self, ConverterBuilderError> {
        self.add_units_file(units)?;
        Ok(self)
    }

    pub fn add_units_file(&mut self, units: UnitsFile) -> Result<&mut Self, ConverterBuilderError> {
        for group in units.quantity {
            // Add all units to an index
            let mut add_units =
                |units: Vec<UnitEntry>, system| -> Result<(), ConverterBuilderError> {
                    for unit in units {
                        let unit = Unit {
                            names: unit.names,
                            symbols: unit.symbols,
                            aliases: unit.aliases,
                            ratio: unit.ratio,
                            difference: unit.difference,
                            physical_quantity: group.quantity,
                            expand_si: unit.expand_si,
                            expanded_units: None,
                            system,
                        };
                        let _id = self.add_unit(unit)?;
                    }
                    Ok(())
                };
            match group.units {
                Units::Unified(units) => add_units(units, None)?,
                Units::BySystem {
                    metric,
                    imperial,
                    unspecified,
                } => {
                    add_units(metric, Some(System::Metric))?;
                    add_units(imperial, Some(System::Imperial))?;
                    add_units(unspecified, None)?;
                }
            };

            // store best units. this will always override
            if let Some(best_units) = group.best {
                if match &best_units {
                    BestUnits::Unified(v) => v.is_empty(),
                    BestUnits::BySystem { metric, imperial } => {
                        metric.is_empty() || imperial.is_empty()
                    }
                } {
                    return Err(ConverterBuilderError::EmptyBest {
                        reason: "empty list of units",
                        quantity: group.quantity,
                    });
                }
                self.best_units[group.quantity] = Some(best_units);
            }
        }

        // Store the extensions to apply them at the end
        if let Some(extend) = units.extend {
            self.extend.push(extend);
        }

        // Join the SI expansion settings
        if let Some(si) = units.si {
            self.si.prefixes = join_prefixes(&mut self.si.prefixes, si.prefixes, si.precedence);
            self.si.symbol_prefixes = join_prefixes(
                &mut self.si.symbol_prefixes,
                si.symbol_prefixes,
                si.precedence,
            );
            self.si.precedence = si.precedence;
        }

        if let Some(default_system) = units.default_system {
            self.default_system = default_system;
        }

        Ok(self)
    }

    pub fn finish(mut self) -> Result<Converter, ConverterBuilderError> {
        // expand the stored units
        for id in 0..self.all_units.len() {
            let unit = &self.all_units[id];
            if unit.expand_si {
                let new_units = expand_si(unit, &self.si)?;
                let mut new_units_ids = EnumMap::<SIPrefix, usize>::default();
                for (prefix, unit) in new_units.into_iter() {
                    new_units_ids[prefix] = self.add_unit(unit)?;
                }
                self.all_units[id].expanded_units = Some(new_units_ids);
            }
        }

        // apply the extend groups
        for extend_group in self.extend {
            let mut to_update = HashSet::new();

            let Extend {
                precedence,
                names,
                symbols,
                aliases,
            } = extend_group;

            for (k, aliases) in aliases {
                let id = self.unit_index.get_unit_id(k.as_str())?;
                self.unit_index.add_aliases(id, aliases.iter().cloned())?;
                join_alias_vec(&mut self.all_units[id].aliases, aliases, precedence);
            }

            for (k, names) in names {
                let id = self.unit_index.get_unit_id(k.as_str())?;
                join_alias_vec(&mut self.all_units[id].names, names, precedence);
                if self.all_units[id].expand_si {
                    to_update.insert(id);
                }
            }

            for (k, symbols) in symbols {
                let id = self.unit_index.get_unit_id(k.as_str())?;
                join_alias_vec(&mut self.all_units[id].symbols, symbols, precedence);
                if self.all_units[id].expand_si {
                    to_update.insert(id);
                }
            }
            // update expansions of the modified units at the end of each group
            // so updates from prior ones are available to the next.
            for id in to_update {
                update_expanded_units(&mut self.unit_index, &mut self.all_units, &self.si, id)?;
            }
        }

        let best = enum_map! {
            q =>  {
                if let Some(best_units) = &self.best_units[q] {
                    BestConversionsStore::new(best_units, &self.unit_index, &mut self.all_units)?
                } else {
                    return Err(ConverterBuilderError::EmptyBest { reason: "no best units given", quantity: q })
                }
            }
        };

        let quantity_index = {
            let mut index: EnumMap<PhysicalQuantity, Vec<usize>> = EnumMap::default();
            for (id, unit) in self.all_units.iter().enumerate() {
                index[unit.physical_quantity].push(id);
            }
            index
        };

        Ok(Converter {
            all_units: self.all_units.into_iter().map(Arc::new).collect(),
            unit_index: self.unit_index,
            quantity_index,
            best,
            default_system: self.default_system,
            temperature_regex: Default::default(),
        })
    }

    fn add_unit(&mut self, unit: Unit) -> Result<usize, ConverterBuilderError> {
        let id = self.all_units.len();
        self.unit_index.add_unit(&unit, id)?;
        self.all_units.push(unit);
        Ok(id)
    }
}

impl BestConversionsStore {
    fn new(
        best_units: &BestUnits,
        unit_index: &UnitIndex,
        all_units: &mut [Unit],
    ) -> Result<Self, ConverterBuilderError> {
        let v = match best_units {
            BestUnits::Unified(names) => {
                Self::Unified(BestConversions::new(names, unit_index, all_units, None)?)
            }
            BestUnits::BySystem { metric, imperial } => Self::BySystem {
                metric: BestConversions::new(metric, unit_index, all_units, Some(System::Metric))?,
                imperial: BestConversions::new(
                    imperial,
                    unit_index,
                    all_units,
                    Some(System::Imperial),
                )?,
            },
        };
        Ok(v)
    }
}

impl BestConversions {
    fn new(
        units: &[String],
        unit_index: &UnitIndex,
        all_units: &mut [Unit],
        system: Option<System>,
    ) -> Result<Self, ConverterBuilderError> {
        let mut units = units
            .iter()
            .map(|n| unit_index.get_unit_id(n))
            .collect::<Result<Vec<_>, _>>()?;

        // TODO do it in other side... it makes no sense to do this in this function
        if let Some(group_system) = system {
            for &unit_id in &units {
                match all_units[unit_id].system {
                    Some(unit_system) => {
                        if group_system != unit_system {
                            return Err(ConverterBuilderError::IncorrectUnitSystem {
                                unit: all_units[unit_id].clone().into(),
                                expected: group_system,
                                got: unit_system,
                            });
                        }
                    }
                    None => all_units[unit_id].system = Some(group_system),
                }
            }
        }

        units.sort_unstable_by(|a, b| {
            let a = &all_units[*a];
            let b = &all_units[*b];
            a.ratio
                .partial_cmp(&b.ratio)
                .unwrap_or(std::cmp::Ordering::Less)
        });

        let mut conversions = Vec::with_capacity(units.len());
        let mut units = units.into_iter();

        let base_unit = units.next().unwrap();
        conversions.push((1.0, base_unit));

        for unit in units {
            let v = convert_f64(1.0, &all_units[unit], &all_units[base_unit]);
            conversions.push((v, unit));
        }

        Ok(Self(conversions))
    }
}

fn update_expanded_units(
    unit_index: &mut UnitIndex,
    all_units: &mut [Unit],
    si: &SI,
    id: usize,
) -> Result<(), ConverterBuilderError> {
    // remove all entries from the unit and expansions from the index
    unit_index.remove_unit_rec(all_units, &all_units[id]);
    // update the expanded units
    let new_units = expand_si(&all_units[id], si)?;
    for (prefix, expanded_unit) in new_units.into_iter() {
        let expanded_id = all_units[id].expanded_units.as_ref().unwrap()[prefix];
        let old_unit_aliases = all_units[expanded_id].aliases.clone();
        all_units[expanded_id] = expanded_unit;
        all_units[expanded_id].aliases = old_unit_aliases;
        unit_index.add_unit(&all_units[expanded_id], expanded_id)?;
    }
    // (re)add the new entries to the index
    unit_index.add_unit(&all_units[id], id)?;
    Ok(())
}

fn join_alias_vec<I: IntoIterator<Item = Arc<str>>>(
    target: &mut Vec<Arc<str>>,
    src: I,
    src_precedence: Precedence,
) {
    match src_precedence {
        Precedence::Before => {
            target.splice(0..0, src);
        }
        Precedence::After => {
            target.extend(src);
        }
        Precedence::Override => {
            target.clear();
            target.extend(src);
        }
    }
}

fn join_prefixes(
    a: &mut Option<EnumMap<SIPrefix, Vec<String>>>,
    b: Option<EnumMap<SIPrefix, Vec<String>>>,
    b_precedence: Precedence,
) -> Option<EnumMap<SIPrefix, Vec<String>>> {
    let a = a.take();
    match (a, b) {
        (None, None) => None,
        (None, Some(v)) | (Some(v), None) => Some(v),
        (Some(mut a), Some(mut b)) => match b_precedence {
            Precedence::Before => {
                a.into_iter().for_each(|(p, v)| b[p].extend(v));
                Some(b)
            }
            Precedence::After => {
                b.into_iter().for_each(|(p, v)| a[p].extend(v));
                Some(a)
            }
            Precedence::Override => Some(b),
        },
    }
}

fn expand_si(unit: &Unit, si: &SI) -> Result<EnumMap<SIPrefix, Unit>, ConverterBuilderError> {
    let (Some(prefixes), Some(symbol_prefixes)) = (&si.prefixes, &si.symbol_prefixes) else {
        return Err(ConverterBuilderError::EmptySIPrefixes);
    };

    let map = enum_map! {
        prefix => {
            let names = prefixes[prefix]
                .iter()
                .flat_map(|p| unit.names.iter().map(move |n| format!("{p}{n}").into()))
                .collect();

            let symbols = symbol_prefixes[prefix]
                .iter()
                .flat_map(|p| unit.symbols.iter().map(move |n| format!("{p}{n}").into()))
                .collect();

            Unit {
                names,
                symbols,
                aliases: Vec::new(),
                ratio: unit.ratio * prefix.ratio(),
                difference: unit.difference,
                physical_quantity: unit.physical_quantity,
                expand_si: false,
                expanded_units: None,
                system: unit.system,
            }
        }
    };

    Ok(map)
}

impl UnitIndex {
    fn remove_unit(&mut self, unit: &Unit) {
        for key in unit.all_keys() {
            self.0.remove(key);
        }
    }

    fn remove_unit_rec(&mut self, all_units: &[Unit], unit: &Unit) {
        if let Some(expanded_units) = &unit.expanded_units {
            for (_, expanded) in expanded_units {
                self.remove_unit_rec(all_units, &all_units[*expanded]);
            }
        }
        self.remove_unit(unit);
    }

    fn add_aliases(
        &mut self,
        unit_id: usize,
        aliases: impl IntoIterator<Item = Arc<str>>,
    ) -> Result<(), ConverterBuilderError> {
        for alias in aliases {
            if self.0.insert(Arc::clone(&alias), unit_id).is_some() {
                return Err(ConverterBuilderError::DuplicateUnit {
                    name: alias.to_string(),
                });
            }
        }

        Ok(())
    }
    fn add_unit(&mut self, unit: &Unit, id: usize) -> Result<usize, ConverterBuilderError> {
        let mut added = 0;
        for key in unit.all_keys() {
            if key.trim().is_empty() {
                return Err(ConverterBuilderError::EmptyUnitKey {
                    unit: unit.clone().into(),
                });
            }
            let maybe_other = self.0.insert(Arc::clone(key), id);
            if maybe_other.is_some() {
                return Err(ConverterBuilderError::DuplicateUnit {
                    name: key.to_string(),
                });
            }
            added += 1;
        }
        if added == 0 {
            return Err(ConverterBuilderError::EmptyUnit {
                unit: unit.clone().into(),
            });
        }
        Ok(added)
    }
}

#[derive(Debug, Error)]
pub enum ConverterBuilderError {
    #[error("Duplicate unit: {name}")]
    DuplicateUnit { name: String },

    #[error(transparent)]
    UnknownUnit(#[from] UnknownUnit),

    #[error("Unit without names or symbols in {}", unit.physical_quantity)]
    EmptyUnit { unit: Box<Unit> },

    #[error("Unit where a name, symbol or alias is empty in {}: {}", unit.physical_quantity, unit.names.first().or(unit.symbols.first()).or(unit.aliases.first()).map(|s| s.to_string()).unwrap_or_else(|| "-".to_string()))]
    EmptyUnitKey { unit: Box<Unit> },

    #[error("Best units for '{quantity}' empty: {reason}")]
    EmptyBest {
        reason: &'static str,
        quantity: PhysicalQuantity,
    },

    #[error("No SI prefixes found when expandind SI on a unit")]
    EmptySIPrefixes,

    #[error("Best units' unit incorrect system: in unit '{unit}' expected {expected}, got {got}")]
    IncorrectUnitSystem {
        unit: Box<Unit>,
        expected: System,
        got: System,
    },
}
