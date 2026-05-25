//! KiCad netlist (S-expression) emitter.
//!
//! Takes a validated `Design` and produces the text of a `.net` file
//! that KiCad's PCB editor can import via *File → Import → Netlist…*.
//!
//! Format reference: see Appendix B of `circuitc-v1-spec.md` and the
//! upstream KiCad docs at
//! <https://docs.kicad.org/master/en/eeschema/eeschema.html#netlist-formats>.
//!
//! The output has four sections:
//!
//!   1. `(design …)`     — provenance: source file, date, tool name.
//!   2. `(components …)` — every component flattened from the module tree.
//!   3. `(libparts …)`   — KiCad's "what does this part look like": pin-level description per component.
//!   4. `(nets …)`       — every net with its (component, pin) nodes.
//!
//! Hierarchy is handled by walking `sub_modules` recursively — blinky
//! is flat so the walk is trivial, but the code is ready for nested
//! designs without rework.

use crate::ir::{Component, Design, Module, Net};
use std::fmt::Write;

/// Render a `Design` as KiCad netlist text.
pub fn emit_netlist(design: &Design) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "(export (version \"E\")");

    emit_design_section(&mut out, design);
    emit_components_section(&mut out, &design.root);
    emit_libparts_section(&mut out, &design.root);
    emit_nets_section(&mut out, &design.root);

    let _ = writeln!(out, ")");
    out
}

fn emit_design_section(out: &mut String, design: &Design) {
    let _ = writeln!(out, "  (design");
    let _ = writeln!(out, "    (source \"{}.k\")", design.name);
    let _ = writeln!(out, "    (date \"\")");
    let _ = writeln!(out, "    (tool \"circuitc-converter\"))");
}

fn emit_components_section(out: &mut String, root: &Module) {
    let _ = writeln!(out, "  (components");
    walk_components(root, &mut |c| {
        let _ = writeln!(out, "    (comp (ref \"{}\")", c.reference);
        let _ = writeln!(out, "      (value \"{}\")", c.value);
        let _ = writeln!(
            out,
            "      (footprint \"{}:{}\")",
            c.footprint.library, c.footprint.name
        );
        if let Some(mpn) = &c.mpn {
            let _ = writeln!(out, "      (fields (field (name \"MPN\") \"{mpn}\"))");
        }
        let _ = writeln!(out, "    )");
    });
    let _ = writeln!(out, "  )");
}

fn emit_libparts_section(out: &mut String, root: &Module) {
    // libparts is KiCad's "schema" for each part: which pins it has,
    // their names, their electrical types. We emit one libpart per
    // component for simplicity — a smarter pass would dedupe by
    // (library, value) so two 10kΩ resistors share a libpart.
    let _ = writeln!(out, "  (libparts");
    walk_components(root, &mut |c| {
        let _ = writeln!(
            out,
            "    (libpart (lib \"{}\") (part \"{}\")",
            c.footprint.library, c.value
        );
        let _ = writeln!(out, "      (pins");
        for pin in &c.pins {
            let _ = writeln!(
                out,
                "        (pin (num \"{}\") (name \"{}\") (type \"passive\"))",
                pin.number, pin.name
            );
        }
        let _ = writeln!(out, "      ))");
    });
    let _ = writeln!(out, "  )");
}

fn emit_nets_section(out: &mut String, root: &Module) {
    let _ = writeln!(out, "  (nets");
    let mut code = 1;
    walk_nets(root, &mut |n: &Net| {
        let _ = writeln!(out, "    (net (code \"{code}\") (name \"{}\")", n.name);
        for conn in &n.connections {
            let _ = writeln!(
                out,
                "      (node (ref \"{}\") (pin \"{}\"))",
                conn.component, conn.pin
            );
        }
        let _ = writeln!(out, "    )");
        code += 1;
    });
    let _ = writeln!(out, "  )");
}

fn walk_components<F: FnMut(&Component)>(module: &Module, f: &mut F) {
    for c in &module.components {
        f(c);
    }
    if let Some(subs) = &module.sub_modules {
        for sub in subs {
            walk_components(sub, f);
        }
    }
}

fn walk_nets<F: FnMut(&Net)>(module: &Module, f: &mut F) {
    for n in &module.nets {
        f(n);
    }
    if let Some(subs) = &module.sub_modules {
        for sub in subs {
            walk_nets(sub, f);
        }
    }
}
