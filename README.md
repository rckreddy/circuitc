# circuitc

A circuit-as-configuration toolchain. A PCB design is defined in **KCL**
(Kusion Configuration Language), validated by KCL's type/constraint
system, and compiled by a small **Rust** binary into a **KiCad**
netlist file that KiCad's PCB editor can import for layout.

```
┌──────────────┐        ┌──────────────┐        ┌──────────────┐
│  blinky.k    │  kcl   │ blinky.json  │  rust  │  blinky.net  │
│  (design)    │ ─────▶ │ (validated   │ ─────▶ │  (KiCad      │
│              │        │  IR)         │        │   netlist)   │
└──────────────┘        └──────────────┘        └──────────────┘
       │                       │                       │
       ▼                       ▼                       ▼
  KCL schemas             serde-deserialized      KiCad opens it
  enforce types           Rust structs            and lays out
  & constraints                                   the board
```

The repo is one workspace with two halves that must move together:

| Path           | What lives here                                                  |
|----------------|------------------------------------------------------------------|
| `kcl/schemas/` | The typed model of a PCB design (`Component`, `Net`, `Module`…). |
| `kcl/designs/` | Concrete designs (e.g. `blinky.k`) that instantiate the schemas. |
| `kcl/tests/`   | KCL test cases for the schemas themselves.                       |
| `converter/`   | Rust CLI: deserialises the JSON IR, runs ERC, emits `.net`.      |
| `output/`      | Generated artefacts (gitignored).                                |
| `docs/`        | Design notes and write-ups.                                      |

## Getting started

```bash
devbox shell        # first run installs Rust toolchain + KCL CLI
just all blinky     # KCL → JSON → ERC → KiCad netlist
cat output/blinky.net
```

Then import `output/blinky.net` into KiCad's PCB editor
(*File → Import → Netlist…*) to see R1 and D1 with a ratsnest
connecting them via VCC, LED_ANODE, and GND.

## Pipeline at a glance

1. **`just build-kcl blinky`** runs `kcl run` over `kcl/designs/blinky.k`.
   The schema `check:` blocks enforce constraints (non-empty names,
   reference designators matching `[A-Z][A-Z0-9_]*`, …). Output: a
   validated JSON IR at `output/blinky.json`.
2. **`just build-rust`** compiles the converter to
   `converter/target/release/circuitc-converter`.
3. **`just convert blinky`** deserialises the JSON, runs electrical
   rule checks (duplicate refs, unknown components, floating pins,
   conflicting drivers), and emits the S-expression `.net` file.

The Justfile defines all three plus `test`, `test-kcl`, `fmt`, `lint`,
and `clean`. Run `just` with no arguments to list them.

## See also

- `circuitc-v1-spec.md` — the full project spec this scaffold realises.
- `docs/notes.md` — your design notes as the project grows.
