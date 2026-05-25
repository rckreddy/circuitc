//! KiCad `.kicad_pcb` (board file) emitter.
//!
//! Where `kicad.rs` emits a *netlist* (connectivity only, no positions),
//! this emits a *full board* file. KiCad opens the result with
//! footprints already placed at the coordinates given in
//! `design.placement` (or laid out on a grid if positions are
//! missing) — no manual "spread components" step needed.
//!
//! ## Format & version
//!
//! Targets the KiCad 7 file format (version date `20221018`). Earlier
//! KiCad versions may refuse to open; newer KiCad versions read it
//! fine but may auto-upgrade the format on save.
//!
//! ## Footprint geometry
//!
//! KiCad's `.kicad_pcb` format does *not* resolve footprint
//! references against the host's library at load time — the geometry
//! (pad positions, sizes, layers) must be inlined per footprint
//! block. We carry a small static catalog ([`footprint_geometry`])
//! for the parts blinky uses. Add new footprints there as the
//! project grows; unknown footprints fall back to a generic
//! two-pad SMD shape so the file still loads.
//!
//! ## Path-2 discipline
//!
//! - The IR is backend-agnostic: positions are
//!   `{x, y, rotation, layer: top|bottom}` in mm/degrees. Translation
//!   to KiCad's `F.Cu`/`B.Cu` layer names happens here, not in `ir.rs`.
//! - Auto-placement (the grid fallback when `design.placement` is
//!   `None` or partial) also lives here. The IR stays pure data —
//!   placement decisions are a backend's opinion.

use crate::ir::{Component, Design, Layer, Module, Net, Position};
use std::collections::HashMap;
use std::fmt::Write;

/// Knobs for the emitter. Defaults are sensible for a small board.
pub struct EmitOptions {
    /// Spacing between auto-placed components, in mm.
    pub grid_spacing_mm: f64,
    /// Where on the page the layout origin sits, in mm. KiCad 7
    /// puts page (0,0) at the top-left of the sheet; a non-zero
    /// offset keeps the board visible without scrolling.
    pub page_offset_mm: (f64, f64),
}

impl Default for EmitOptions {
    fn default() -> Self {
        Self {
            grid_spacing_mm: 5.0,
            page_offset_mm: (100.0, 100.0),
        }
    }
}

/// Render a `Design` as a KiCad `.kicad_pcb` board file.
pub fn emit_pcb(design: &Design, opts: &EmitOptions) -> String {
    // Collect nets in deterministic walk order; assign KiCad net codes
    // starting at 1 (code 0 is the reserved "no net" placeholder).
    let nets: Vec<&Net> = all_nets(&design.root);
    let mut net_code: HashMap<&str, usize> = HashMap::new();
    for (i, net) in nets.iter().enumerate() {
        net_code.insert(net.name.as_str(), i + 1);
    }

    // Reverse lookup: (component reference, pin id) → net code.
    // Used per-pad to write the `(net N "name")` clause.
    let mut pin_net: HashMap<(String, String), (usize, String)> = HashMap::new();
    for net in &nets {
        let code = net_code[net.name.as_str()];
        for conn in &net.connections {
            pin_net.insert(
                (conn.component.clone(), conn.pin.to_string()),
                (code, net.name.clone()),
            );
        }
    }

    let mut out = String::new();
    // Format version targets KiCad 8/9/10 (20240108). The older
    // 20221018 date is parsed fine by modern KiCad but triggers
    // compat behaviour that suppresses property text rendering.
    let _ = writeln!(
        out,
        "(kicad_pcb (version 20240108) (generator \"circuitc\")"
    );
    write_general(&mut out);
    write_paper(&mut out);
    write_layers(&mut out);
    write_setup(&mut out);
    write_nets_section(&mut out, &nets);

    // Auto-placement counter — only used for components missing from
    // `design.placement`. Increments only on the auto path so the
    // grid stays contiguous regardless of how many positions are
    // explicit.
    let mut auto_index = 0usize;
    // Deterministic uuid generator so snapshots are reproducible.
    // KiCad needs uuids on every footprint / property / pad to render
    // them; absence triggers the compat behaviour above.
    let mut uuids = UuidGen::new();
    for c in all_components(&design.root) {
        let local = resolve_position(design, &c.reference, opts, &mut auto_index);
        // Translate local board coordinates to absolute page coordinates
        // by adding the page offset. Reuse `Position` rather than
        // passing each axis separately — keeps `write_footprint` under
        // clippy's argument-count threshold.
        let absolute = Position {
            x: local.x + opts.page_offset_mm.0,
            y: local.y + opts.page_offset_mm.1,
            rotation: local.rotation,
            layer: local.layer,
        };
        let geometry = footprint_geometry(&c.footprint.library, &c.footprint.name);
        write_footprint(&mut out, c, &absolute, geometry, &pin_net, &mut uuids);
    }

    let _ = writeln!(out, ")");
    out
}

/// Counter-backed uuid generator. Real KiCad emits random v4 uuids;
/// we use a deterministic sequence so snapshot tests are reproducible.
/// KiCad only cares that uuids are unique within the file.
struct UuidGen {
    counter: u64,
}

impl UuidGen {
    fn new() -> Self {
        Self { counter: 0 }
    }

    fn next(&mut self) -> String {
        self.counter += 1;
        format!("00000000-0000-0000-0000-{:012x}", self.counter)
    }
}

// === Sections ===

fn write_general(out: &mut String) {
    let _ = writeln!(out, "  (general");
    let _ = writeln!(out, "    (thickness 1.6)");
    let _ = writeln!(out, "  )");
}

fn write_paper(out: &mut String) {
    let _ = writeln!(out, "  (paper \"A4\")");
}

fn write_layers(out: &mut String) {
    // The KiCad layer stack is largely invariant for a 2-layer board.
    // Hardcoding it here keeps the emitter self-contained; a Phase-5
    // multi-layer extension would parameterise this block.
    let _ = writeln!(out, "  (layers");
    for line in LAYER_STACK {
        let _ = writeln!(out, "    {line}");
    }
    let _ = writeln!(out, "  )");
}

fn write_setup(out: &mut String) {
    let _ = writeln!(out, "  (setup");
    let _ = writeln!(out, "    (pad_to_mask_clearance 0)");
    let _ = writeln!(out, "  )");
}

fn write_nets_section(out: &mut String, nets: &[&Net]) {
    // Net 0 is KiCad's reserved "unconnected" placeholder.
    let _ = writeln!(out, "  (net 0 \"\")");
    for (i, net) in nets.iter().enumerate() {
        let _ = writeln!(out, "  (net {} \"{}\")", i + 1, net.name);
    }
}

fn write_footprint(
    out: &mut String,
    component: &Component,
    placement: &Position,
    geom: &FootprintGeom,
    pin_net: &HashMap<(String, String), (usize, String)>,
    uuids: &mut UuidGen,
) {
    let kicad_layer = placement.layer.kicad_copper();
    let _ = writeln!(
        out,
        "  (footprint \"{}:{}\" (layer \"{}\")",
        component.footprint.library, component.footprint.name, kicad_layer
    );
    let _ = writeln!(out, "    (uuid \"{}\")", uuids.next());
    let _ = writeln!(
        out,
        "    (at {} {} {})",
        placement.x, placement.y, placement.rotation
    );
    let _ = writeln!(out, "    (descr \"{}\")", geom.descr);
    // `attr smd` marks this footprint as SMD so KiCad's tools treat
    // it correctly (placement file generation, courtyards, etc.).
    let _ = writeln!(out, "    (attr smd)");

    // Reference and value text use KiCad's standard library defaults
    // for 0402 silkscreen (0.5 mm, 0.075 mm stroke) so the property
    // text visually matches KiCad's auto-rendered pad net labels —
    // both end up at the same scale.
    //
    // The `unlocked` flag inside `(at …)` and the per-property
    // `(uuid …)` are what makes KiCad 8/9/10 actually *render* the
    // property as visible silkscreen text. Without them the property
    // is parsed (right-click → Properties shows the value) but the
    // text is suppressed by compat rules.
    let silk = placement.layer.kicad_silkscreen();
    let fab = placement.layer.kicad_fab();
    let _ = writeln!(
        out,
        "    (property \"Reference\" \"{}\" (at 0 -1.0 0 unlocked) (layer \"{}\") (uuid \"{}\") (effects (font (size 0.5 0.5) (thickness 0.075))))",
        component.reference, silk, uuids.next()
    );
    let _ = writeln!(
        out,
        "    (property \"Value\" \"{}\" (at 0 1.0 0 unlocked) (layer \"{}\") (uuid \"{}\") (effects (font (size 0.5 0.5) (thickness 0.075))))",
        component.value, fab, uuids.next()
    );

    let pad_layers = placement.layer.kicad_pad_layers();
    for pad in geom.pads {
        let net_clause = pin_net
            .get(&(component.reference.clone(), pad.number.to_string()))
            .map(|(code, name)| format!(" (net {code} \"{name}\")"))
            .unwrap_or_default();
        let _ = writeln!(
            out,
            "    (pad \"{}\" smd roundrect (at {} {}) (size {} {}) (layers {}) (roundrect_rratio 0.25) (uuid \"{}\"){})",
            pad.number, pad.x, pad.y, pad.size_x, pad.size_y, pad_layers, uuids.next(), net_clause
        );
    }
    let _ = writeln!(out, "  )");
}

// === Layer translation ===
//
// KCL/IR speak in "top" / "bottom"; KiCad speaks in `F.*` / `B.*`.
// All the F/B mapping lives in this impl so the rest of the file
// stays layer-agnostic.

impl Layer {
    fn kicad_copper(self) -> &'static str {
        match self {
            Layer::Top => "F.Cu",
            Layer::Bottom => "B.Cu",
        }
    }
    fn kicad_silkscreen(self) -> &'static str {
        match self {
            Layer::Top => "F.SilkS",
            Layer::Bottom => "B.SilkS",
        }
    }
    fn kicad_fab(self) -> &'static str {
        match self {
            Layer::Top => "F.Fab",
            Layer::Bottom => "B.Fab",
        }
    }
    fn kicad_pad_layers(self) -> &'static str {
        match self {
            Layer::Top => "\"F.Cu\" \"F.Paste\" \"F.Mask\"",
            Layer::Bottom => "\"B.Cu\" \"B.Paste\" \"B.Mask\"",
        }
    }
}

// === Footprint catalog ===

struct PadGeom {
    number: &'static str,
    x: f64,
    y: f64,
    size_x: f64,
    size_y: f64,
}

struct FootprintGeom {
    descr: &'static str,
    pads: &'static [PadGeom],
}

const R_0402: FootprintGeom = FootprintGeom {
    descr: "Resistor SMD 0402 (1005 metric)",
    pads: &[
        PadGeom { number: "1", x: -0.47, y: 0.0, size_x: 0.55, size_y: 0.6 },
        PadGeom { number: "2", x: 0.47, y: 0.0, size_x: 0.55, size_y: 0.6 },
    ],
};

const LED_0402: FootprintGeom = FootprintGeom {
    descr: "LED SMD 0402 (1005 metric)",
    pads: &[
        PadGeom { number: "1", x: -0.47, y: 0.0, size_x: 0.55, size_y: 0.6 },
        PadGeom { number: "2", x: 0.47, y: 0.0, size_x: 0.55, size_y: 0.6 },
    ],
};

const FALLBACK_2PAD: FootprintGeom = FootprintGeom {
    descr: "Generic 2-pad SMD (fallback for unknown footprints)",
    pads: &[
        PadGeom { number: "1", x: -1.0, y: 0.0, size_x: 1.0, size_y: 1.0 },
        PadGeom { number: "2", x: 1.0, y: 0.0, size_x: 1.0, size_y: 1.0 },
    ],
};

/// Look up the inlined pad geometry for a given KiCad footprint
/// reference. Falls back to a generic 2-pad SMD shape for anything
/// unknown — the file will still load, but you'll want to add the
/// real geometry here before relying on it.
fn footprint_geometry(library: &str, name: &str) -> &'static FootprintGeom {
    match (library, name) {
        ("Resistor_SMD", "R_0402_1005Metric") => &R_0402,
        ("LED_SMD", "LED_0402_1005Metric") => &LED_0402,
        _ => &FALLBACK_2PAD,
    }
}

// === Placement resolution ===

fn resolve_position(
    design: &Design,
    reference: &str,
    opts: &EmitOptions,
    auto_index: &mut usize,
) -> Position {
    if let Some(placement) = &design.placement {
        if let Some(pos) = placement.get(reference) {
            return *pos;
        }
    }
    let pos = Position {
        x: (*auto_index as f64) * opts.grid_spacing_mm,
        y: 0.0,
        rotation: 0.0,
        layer: Layer::Top,
    };
    *auto_index += 1;
    pos
}

// === Module walks (recursive so sub_modules work) ===

fn all_components(module: &Module) -> Vec<&Component> {
    let mut out = Vec::new();
    walk_components_into(module, &mut out);
    out
}

fn walk_components_into<'a>(module: &'a Module, out: &mut Vec<&'a Component>) {
    for c in &module.components {
        out.push(c);
    }
    if let Some(subs) = &module.sub_modules {
        for sub in subs {
            walk_components_into(sub, out);
        }
    }
}

fn all_nets(module: &Module) -> Vec<&Net> {
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

// === Layer stack constant ===

const LAYER_STACK: &[&str] = &[
    "(0 \"F.Cu\" signal)",
    "(31 \"B.Cu\" signal)",
    "(32 \"B.Adhes\" user \"B.Adhesive\")",
    "(33 \"F.Adhes\" user \"F.Adhesive\")",
    "(34 \"B.Paste\" user)",
    "(35 \"F.Paste\" user)",
    "(36 \"B.SilkS\" user \"B.Silkscreen\")",
    "(37 \"F.SilkS\" user \"F.Silkscreen\")",
    "(38 \"B.Mask\" user)",
    "(39 \"F.Mask\" user)",
    "(40 \"Dwgs.User\" user \"User.Drawings\")",
    "(41 \"Cmts.User\" user \"User.Comments\")",
    "(42 \"Eco1.User\" user \"User.Eco1\")",
    "(43 \"Eco2.User\" user \"User.Eco2\")",
    "(44 \"Edge.Cuts\" user)",
    "(45 \"Margin\" user)",
    "(46 \"B.CrtYd\" user \"B.Courtyard\")",
    "(47 \"F.CrtYd\" user \"F.Courtyard\")",
    "(48 \"B.Fab\" user)",
    "(49 \"F.Fab\" user)",
];
