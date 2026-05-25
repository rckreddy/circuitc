//! circuitc-converter library crate.
//!
//! Hosts the three pure modules — `ir` (data shapes), `kicad` (netlist
//! emitter), `erc` (electrical rule checks) — so they're reachable
//! both from the CLI binary (`main.rs`) and from integration tests
//! under `tests/`. The CLI itself is intentionally thin; everything
//! testable lives here.
//!
//! ## Pipeline
//!
//! ```text
//! JSON (from `kcl run`)
//!   └─▶ serde_json::from_str::<ir::KclOutput>
//!         └─▶ erc::check(&design)        // returns Vec<ErcError>
//!         └─▶ kicad::emit_netlist(&design) -> String  // S-expression
//!               └─▶ write to disk
//! ```

pub mod erc;
pub mod ir;
pub mod kicad;
pub mod kicad_pcb;
