//! CLI entry point for `circuitc-converter`.
//!
//! Reads JSON IR from `--input`, deserialises it into the lib's
//! `ir::KclOutput`, runs ERC unless `--skip-erc` is set, emits a
//! KiCad netlist via `kicad::emit_netlist`, writes it to `--output`.
//!
//! Convention (notebook skill 174):
//!   - data (none here — netlist goes to a file, not stdout) → stdout
//!   - progress / status messages → stderr-equivalent (`eprintln!` for
//!     errors; `println!` for the success/progress lines — these get
//!     piped to stderr by the Justfile via `>&2` if you need to
//!     separate them in CI).
//!   - exit 0 on success, exit 1 on ERC failure or any I/O error.
//!
//! All error handling goes through `anyhow::Context` so the final
//! error message names the file or step that failed (notebook
//! skill 45).

use anyhow::{Context, Result};
use circuitc_converter::{erc, ir, kicad, kicad_pcb};
use clap::{Parser, ValueEnum};
use std::path::PathBuf;

/// Which KiCad artefact to emit.
///
/// `Net` is the schematic-style netlist (`.net`): connectivity only,
/// imported via *File → Import → Netlist…*. Components land stacked
/// at the origin and you spread them by hand.
///
/// `Pcb` is the board file (`.kicad_pcb`): full layout with
/// component positions, opened directly. Uses the `placement` block
/// in the design (or auto-places on a grid if absent).
#[derive(Copy, Clone, Default, PartialEq, Eq, ValueEnum)]
enum Format {
    #[default]
    Net,
    Pcb,
}

#[derive(Parser)]
#[command(name = "circuitc-converter")]
#[command(about = "Convert KCL circuit JSON IR to a KiCad netlist or board file")]
struct Cli {
    /// Path to the KCL-generated JSON file (output of `kcl run`).
    #[arg(short, long)]
    input: PathBuf,

    /// Output path for the emitted file.
    #[arg(short, long)]
    output: PathBuf,

    /// Which KiCad artefact to emit. Default `net` for backward
    /// compatibility; `pcb` writes a full `.kicad_pcb` board file.
    #[arg(short, long, value_enum, default_value_t = Format::Net)]
    format: Format,

    /// Skip electrical rule checks. Not recommended outside debugging.
    #[arg(long)]
    skip_erc: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let json = std::fs::read_to_string(&cli.input)
        .with_context(|| format!("reading {}", cli.input.display()))?;
    let parsed: ir::KclOutput =
        serde_json::from_str(&json).context("parsing KCL JSON output")?;
    let design = parsed.design;

    println!(
        "📐 Loaded design '{}' with {} component(s) and {} net(s) at root",
        design.name,
        design.root.components.len(),
        design.root.nets.len(),
    );

    if !cli.skip_erc {
        let errors = erc::check(&design);
        if !errors.is_empty() {
            eprintln!("\n❌ ERC failed with {} error(s):", errors.len());
            for e in &errors {
                eprintln!("   - {e}");
            }
            std::process::exit(1);
        }
        println!("✅ ERC passed");
    }

    let output = match cli.format {
        Format::Net => kicad::emit_netlist(&design),
        Format::Pcb => kicad_pcb::emit_pcb(&design, &kicad_pcb::EmitOptions::default()),
    };
    std::fs::write(&cli.output, &output)
        .with_context(|| format!("writing {}", cli.output.display()))?;

    println!(
        "✅ Wrote {} ({} bytes, format={})",
        cli.output.display(),
        output.len(),
        match cli.format {
            Format::Net => "net",
            Format::Pcb => "pcb",
        },
    );
    Ok(())
}
