# circuitc task runner.
#
# Parameterised on `design="blinky"` so the same recipes serve future
# designs (voltage_divider, etc.) without copy-paste. Run `just` with
# no arguments to list everything.

default:
    @just --list

# Compile a KCL design to its JSON IR.
build-kcl design="blinky":
    @mkdir -p output
    cd kcl && kcl run designs/{{design}}.k -o ../output/{{design}}.json --format json

# Build the Rust converter in release mode.
build-rust:
    cd converter && cargo build --release

# Convert a JSON IR to a KiCad netlist (.net).
convert design="blinky":
    ./converter/target/release/circuitc-converter \
        --input output/{{design}}.json \
        --output output/{{design}}.net

# Convert a JSON IR to a KiCad board file (.kicad_pcb) with positions.
# Opens directly in KiCad's PCB editor — no manual spreading.
pcb design="blinky":
    ./converter/target/release/circuitc-converter \
        --input output/{{design}}.json \
        --output output/{{design}}.kicad_pcb \
        --format pcb

# Full pipeline: KCL → JSON → KiCad netlist.
all design="blinky": (build-kcl design) build-rust (convert design)
    @echo "✅ Generated output/{{design}}.net"

# Full pipeline: KCL → JSON → KiCad board file with placements.
all-pcb design="blinky": (build-kcl design) build-rust (pcb design)
    @echo "✅ Generated output/{{design}}.kicad_pcb"

# Run Rust tests (includes the netlist snapshot test).
test:
    cd converter && cargo test

# Run KCL schema tests.
test-kcl:
    cd kcl && kcl test ./tests/...

# Format everything.
fmt:
    cd converter && cargo fmt
    cd kcl && kcl fmt ./...

# Lint everything; treat warnings as errors so CI catches them.
lint:
    cd converter && cargo clippy --all-targets -- -D warnings
    cd kcl && kcl lint ./...

# Remove all generated artefacts.
clean:
    rm -rf output/* converter/target kcl/.kclvm
