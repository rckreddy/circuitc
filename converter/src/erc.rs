//! Electrical Rule Checks.
//!
//! Catch design mistakes that the KCL `check:` blocks can't catch
//! because they need *cross-element* knowledge (a net referencing a
//! component the other side of the file, two outputs on the same
//! net, …). Each `check:` block sees one schema instance at a time;
//! ERC sees the whole design.
//!
//! The five checks here are deliberately the minimum useful set:
//!
//!   1. Duplicate reference designators (R1 declared twice).
//!   2. Net referencing an unknown component.
//!   3. Net referencing a pin that doesn't exist on its component.
//!   4. Pin declared on a component but never connected to a net.
//!   5. Two output-driving pins on the same signal net.
//!
//! Phase-5 exercise in the spec: pick one of these and re-implement
//! it as a KCL `check:` block. The interesting cases (cross-element
//! ones) you'll find can't be expressed in KCL today — and *that* is
//! the comparison the project is trying to make concrete.
//!
//! Uses `thiserror` so the `ErcError` enum gets a derived `Display`
//! and `Error` impl (notebook skill 44).

use crate::ir::{Component, Design, Module, Net, NetKind, PinDirection};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ErcError {
    #[error("duplicate reference designator: {0}")]
    DuplicateReference(String),

    #[error("net '{net}' references unknown component '{component}'")]
    UnknownComponent { net: String, component: String },

    #[error("net '{net}' references pin '{pin}' on '{component}' which doesn't exist")]
    UnknownPin {
        net: String,
        component: String,
        pin: String,
    },

    #[error("pin '{component}.{pin}' is not connected to any net")]
    FloatingPin { component: String, pin: String },

    #[error("net '{net}' has multiple output drivers: {components:?}")]
    ConflictingDrivers {
        net: String,
        components: Vec<String>,
    },
}

/// Run every ERC pass over `design` and return all violations.
///
/// Returns an empty `Vec` if the design is clean. The CLI exits with
/// status 1 if this returns anything non-empty.
pub fn check(design: &Design) -> Vec<ErcError> {
    let mut errors = Vec::new();

    // Walk the module tree once to gather every component, keyed by
    // reference designator. Catching duplicate refs during *insertion*
    // is the only correct moment — a HashMap by definition can't hold
    // two entries with the same key, so checking the keys afterwards
    // would always report zero.
    let mut all_components: HashMap<String, &Component> = HashMap::new();
    let mut seen_refs: HashSet<String> = HashSet::new();
    collect_components(&design.root, &mut all_components, &mut seen_refs, &mut errors);

    let all_nets = collect_nets(&design.root);

    // Checks 2 & 3: every net endpoint must point at a real
    // component and a real pin on that component. Track which pins
    // are connected so check 4 (floating pins) can fire.
    let mut connected: HashSet<(String, String)> = HashSet::new();
    for net in &all_nets {
        for conn in &net.connections {
            match all_components.get(&conn.component) {
                None => errors.push(ErcError::UnknownComponent {
                    net: net.name.clone(),
                    component: conn.component.clone(),
                }),
                Some(component) => {
                    let pin_str = conn.pin.to_string();
                    let exists = component
                        .pins
                        .iter()
                        .any(|p| p.number.to_string() == pin_str);
                    if exists {
                        connected.insert((conn.component.clone(), pin_str));
                    } else {
                        errors.push(ErcError::UnknownPin {
                            net: net.name.clone(),
                            component: conn.component.clone(),
                            pin: pin_str,
                        });
                    }
                }
            }
        }
    }

    // Check 4: every declared pin must appear in at least one net.
    for (reference, component) in &all_components {
        for pin in &component.pins {
            let key = (reference.clone(), pin.number.to_string());
            if !connected.contains(&key) {
                errors.push(ErcError::FloatingPin {
                    component: reference.clone(),
                    pin: pin.number.to_string(),
                });
            }
        }
    }

    // Check 5: multiple output drivers on a signal net. Power/ground
    // nets are exempted because multiple regulators or ground returns
    // are normal and expected.
    for net in &all_nets {
        if matches!(net.kind, NetKind::Power | NetKind::Ground) {
            continue;
        }
        let drivers: Vec<String> = net
            .connections
            .iter()
            .filter_map(|conn| {
                let component = all_components.get(&conn.component)?;
                let pin = component
                    .pins
                    .iter()
                    .find(|p| p.number.to_string() == conn.pin.to_string())?;
                matches!(pin.direction, PinDirection::Output | PinDirection::PowerOut)
                    .then(|| conn.component.clone())
            })
            .collect();
        if drivers.len() > 1 {
            errors.push(ErcError::ConflictingDrivers {
                net: net.name.clone(),
                components: drivers,
            });
        }
    }

    errors
}

fn collect_components<'a>(
    module: &'a Module,
    out: &mut HashMap<String, &'a Component>,
    seen: &mut HashSet<String>,
    errors: &mut Vec<ErcError>,
) {
    for c in &module.components {
        if !seen.insert(c.reference.clone()) {
            errors.push(ErcError::DuplicateReference(c.reference.clone()));
        }
        out.insert(c.reference.clone(), c);
    }
    if let Some(subs) = &module.sub_modules {
        for sub in subs {
            collect_components(sub, out, seen, errors);
        }
    }
}

fn collect_nets(module: &Module) -> Vec<&Net> {
    let mut out = Vec::new();
    walk_nets_into(module, &mut out);
    out
}

fn walk_nets_into<'a>(module: &'a Module, out: &mut Vec<&'a Net>) {
    for n in &module.nets {
        out.push(n);
    }
    if let Some(subs) = &module.sub_modules {
        for sub in subs {
            walk_nets_into(sub, out);
        }
    }
}
