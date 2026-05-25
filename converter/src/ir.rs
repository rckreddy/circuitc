//! Rust IR mirroring the KCL schemas in `kcl/schemas/circuit.k`.
//!
//! **Lockstep invariant:** every field and enum variant in this file
//! must correspond to one in the KCL schema. When you add a field on
//! either side, add it on the other in the same commit. `serde` is
//! configured leniently (`#[serde(default)]` on every optional) so
//! that *missing* JSON keys are fine, but field-name typos will fail
//! to parse — that's the contract this file defends.
//!
//! Patterns applied (see `engineering-notebook/rust/`):
//!   - Skill 143 (enum representations): `PinDirection` and `NetKind`
//!     use `#[serde(rename_all = "lowercase")]` to match the
//!     lowercase string literals in the KCL schema. `PinId` uses
//!     `#[serde(untagged)]` because KCL emits a raw number *or* a
//!     raw string with no discriminator tag.
//!   - Skill 150 (serde best practices): `#[serde(default)]` on every
//!     optional field; `Default` impls on enums that have a default
//!     variant in KCL.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A pin identifier: either numeric (`1`, `2`, …) or symbolic
/// (`"A"`, `"K"`, `"A1"`). KCL accepts both, so we accept both.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum PinId {
    Number(u32),
    Name(String),
}

impl std::fmt::Display for PinId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PinId::Number(n) => write!(f, "{n}"),
            PinId::Name(s) => write!(f, "{s}"),
        }
    }
}

/// Electrical direction of a pin. Used by ERC to flag e.g. two outputs
/// driving the same net.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum PinDirection {
    Input,
    Output,
    Bidir,
    PowerIn,
    PowerOut,
    #[default]
    Passive,
}

/// Physical land pattern on the PCB. Maps directly to a KiCad
/// `library:name` footprint reference.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Footprint {
    pub library: String,
    pub name: String,
}

/// A single electrical pin on a component.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Pin {
    pub name: String,
    pub number: PinId,
    #[serde(default)]
    pub direction: PinDirection,
    #[serde(default)]
    pub voltage_rating: Option<f64>,
}

/// A physical part on the board.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Component {
    pub reference: String,
    pub value: String,
    pub footprint: Footprint,
    pub pins: Vec<Pin>,
    #[serde(default)]
    pub manufacturer: Option<String>,
    #[serde(default)]
    pub mpn: Option<String>,
    #[serde(default)]
    pub datasheet: Option<String>,
}

/// One endpoint of a net: a specific pin on a specific component.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Connection {
    pub component: String,
    pub pin: PinId,
}

/// Classification of a net. ERC treats `Power` and `Ground` specially —
/// e.g. multiple drivers on a signal net is an error, but on a power
/// rail it's the norm.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum NetKind {
    Power,
    Ground,
    #[default]
    Signal,
    Analog,
    Differential,
}

/// An electrical net joining one or more pins.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Net {
    pub name: String,
    pub connections: Vec<Connection>,
    #[serde(default)]
    pub kind: NetKind,
    #[serde(default)]
    pub voltage: Option<f64>,
}

/// A logical grouping of components and nets. Modules can nest via
/// `sub_modules`, though the blinky example is flat.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Module {
    pub name: String,
    #[serde(default)]
    pub components: Vec<Component>,
    #[serde(default)]
    pub nets: Vec<Net>,
    #[serde(default)]
    pub sub_modules: Option<Vec<Module>>,
}

/// Which side of the PCB a footprint lives on. Backend-agnostic on
/// purpose — every PCB tool calls these "top" and "bottom" even when
/// their internal layer names differ (KiCad: `F.Cu`/`B.Cu`,
/// Altium: `Top Layer`/`Bottom Layer`). The KiCad backend translates
/// at emit time.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Layer {
    #[default]
    Top,
    Bottom,
}

/// Physical placement of a single footprint. Coordinates are in
/// millimetres, rotation in degrees CCW from the part's library
/// orientation. The origin (0, 0) sits at the centre of the board
/// outline by convention, but the IR doesn't enforce that — a
/// backend may translate everything before emitting.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Position {
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub rotation: f64,
    #[serde(default)]
    pub layer: Layer,
}

/// The top-level circuit design.
///
/// `placement` is a **sidecar** keyed by reference designator, not a
/// field on `Component`. This is deliberate: the logical circuit
/// (`root`) is one thing; a *physical realisation* of it on a board
/// is another. Different boards (dev board, production, breadboard
/// adapter) can share one `root` and differ only in `placement`. A
/// design without a `placement` is electrically complete; the backend
/// auto-places on a grid.
///
/// `BTreeMap`, not `HashMap`, so iteration order is deterministic and
/// snapshot tests don't flake.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Design {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub root: Module,
    #[serde(default)]
    pub placement: Option<BTreeMap<String, Position>>,
}

/// Wrapper around `kcl run`'s output. KCL emits every top-level binding
/// in the design file as a key in one JSON object; the converter only
/// reads `design`, which by convention is the top-level `Design` value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KclOutput {
    pub design: Design,
}
