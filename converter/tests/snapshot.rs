//! Snapshot test for the KiCad netlist emitter.
//!
//! We build a `Design` fixture in-code rather than reading
//! `output/blinky.json` — that file is gitignored and only exists
//! after `just build-kcl blinky` has run, so a test depending on it
//! would be flaky in CI. The fixture mirrors `kcl/designs/blinky.k`
//! exactly; if the two ever drift, the snapshot test will catch it.
//!
//! Run with `cargo test`. First run writes
//! `tests/snapshots/snapshot__blinky_netlist.snap`; review it once,
//! commit it, then subsequent runs diff against it. Update with
//! `cargo insta review` after intentional emitter changes.

use circuitc_converter::ir::{
    Component, Connection, Design, Footprint, Layer, Module, Net, NetKind, Pin, PinDirection,
    PinId, Position,
};
use circuitc_converter::{erc, kicad, kicad_pcb};
use std::collections::BTreeMap;

fn blinky_fixture() -> Design {
    Design {
        name: "blinky".into(),
        description: Some("Single-LED indicator powered from 3.3 V".into()),
        root: Module {
            name: "blinky".into(),
            components: vec![
                Component {
                    reference: "R1".into(),
                    value: "1kohm".into(),
                    footprint: Footprint {
                        library: "Resistor_SMD".into(),
                        name: "R_0402_1005Metric".into(),
                    },
                    pins: vec![
                        Pin {
                            name: "1".into(),
                            number: PinId::Number(1),
                            direction: PinDirection::Passive,
                            voltage_rating: None,
                        },
                        Pin {
                            name: "2".into(),
                            number: PinId::Number(2),
                            direction: PinDirection::Passive,
                            voltage_rating: None,
                        },
                    ],
                    manufacturer: Some("Yageo".into()),
                    mpn: Some("RC0402FR-071KL".into()),
                    datasheet: None,
                },
                Component {
                    reference: "D1".into(),
                    value: "LED_RED".into(),
                    footprint: Footprint {
                        library: "LED_SMD".into(),
                        name: "LED_0402_1005Metric".into(),
                    },
                    pins: vec![
                        Pin {
                            name: "A".into(),
                            number: PinId::Number(1),
                            direction: PinDirection::Passive,
                            voltage_rating: None,
                        },
                        Pin {
                            name: "K".into(),
                            number: PinId::Number(2),
                            direction: PinDirection::Passive,
                            voltage_rating: None,
                        },
                    ],
                    manufacturer: Some("Wurth".into()),
                    mpn: Some("150040RS75000".into()),
                    datasheet: None,
                },
            ],
            nets: vec![
                Net {
                    name: "VCC".into(),
                    connections: vec![Connection {
                        component: "R1".into(),
                        pin: PinId::Number(1),
                    }],
                    kind: NetKind::Power,
                    voltage: Some(3.3),
                },
                Net {
                    name: "LED_ANODE".into(),
                    connections: vec![
                        Connection {
                            component: "R1".into(),
                            pin: PinId::Number(2),
                        },
                        Connection {
                            component: "D1".into(),
                            pin: PinId::Number(1),
                        },
                    ],
                    kind: NetKind::Signal,
                    voltage: None,
                },
                Net {
                    name: "GND".into(),
                    connections: vec![Connection {
                        component: "D1".into(),
                        pin: PinId::Number(2),
                    }],
                    kind: NetKind::Ground,
                    voltage: Some(0.0),
                },
            ],
            sub_modules: None,
        },
        placement: Some(BTreeMap::from([
            (
                "R1".into(),
                Position { x: 0.0, y: 0.0, rotation: 0.0, layer: Layer::Top },
            ),
            (
                "D1".into(),
                Position { x: 5.0, y: 0.0, rotation: 0.0, layer: Layer::Top },
            ),
        ])),
    }
}

#[test]
fn blinky_netlist() {
    let netlist = kicad::emit_netlist(&blinky_fixture());
    insta::assert_snapshot!(netlist);
}

#[test]
fn blinky_pcb() {
    let pcb = kicad_pcb::emit_pcb(&blinky_fixture(), &kicad_pcb::EmitOptions::default());
    insta::assert_snapshot!(pcb);
}

#[test]
fn blinky_passes_erc() {
    let errors = erc::check(&blinky_fixture());
    assert!(
        errors.is_empty(),
        "expected clean ERC, got {errors:#?}",
    );
}
