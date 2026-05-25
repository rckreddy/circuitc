"""
Core schemas for describing a PCB design.

This file is the source of truth for what a circuit *is*. A `Design`
contains a hierarchy of `Module`s; each `Module` holds `Component`s
(physical parts like resistors and ICs) and `Net`s (electrical
connections between component pins). Everything else in the toolchain
(Rust IR, ERC, KiCad emitter) downstreams from these shapes.

Two invariants worth keeping in mind:

  1. The Rust structs in `converter/src/ir.rs` must move in lockstep
     with these schemas. A new field here is a new field there.

  2. Prefer pushing validation into `check:` blocks over re-implementing
     it in Rust. The whole pedagogical point of KCL over Starlark is
     that constraints are declarative and the compiler enforces them
     before the JSON IR ever reaches a downstream tool.
"""

import regex

schema Pin:
    """A single electrical pin on a component."""
    name: str
    number: int | str          # "1", 1, "A1" all valid — KCL accepts either
    direction: "input" | "output" | "bidir" | "power_in" | "power_out" | "passive" = "passive"
    voltage_rating?: float     # Maximum voltage this pin can tolerate, in volts

    check:
        len(name) > 0, "pin name cannot be empty"

schema Footprint:
    """Physical land pattern on the PCB. Maps to a KiCad footprint library entry."""
    library: str               # e.g. "Resistor_SMD"
    name: str                  # e.g. "R_0402_1005Metric"

    check:
        len(library) > 0, "footprint library required"
        len(name) > 0, "footprint name required"

schema Component:
    """A physical part on the board (resistor, capacitor, IC, connector, …)."""
    reference: str             # R1, C3, U2 — unique within a design
    value: str                 # "10kohm", "STM32F405RGT6", "LED_RED"
    footprint: Footprint
    pins: [Pin]
    manufacturer?: str
    mpn?: str                  # Manufacturer Part Number
    datasheet?: str

    check:
        len(reference) > 0, "reference designator required (e.g. R1, C3, U2)"
        len(pins) > 0, "component must declare at least one pin"
        # Reference must start with a capital letter and contain only A–Z, 0–9, _.
        # This is what KiCad expects and what every PCB tool in the wild assumes.
        regex.match(reference, r"^[A-Z][A-Z0-9_]*$"), \
            "reference must match [A-Z][A-Z0-9_]* (e.g. R1, U2, J3)"

schema Connection:
    """A single (component, pin) endpoint of a net."""
    component: str             # reference designator, e.g. "R1"
    pin: int | str             # pin number or name

schema Net:
    """An electrical connection joining one or more component pins."""
    name: str
    connections: [Connection]
    kind?: "power" | "ground" | "signal" | "analog" | "differential" = "signal"
    voltage?: float            # Nominal voltage for power nets (in volts)

    check:
        len(name) > 0, "net name required"
        len(connections) >= 1, "net must connect at least one pin"

schema Module:
    """A logical grouping of components and nets. Can be nested via `sub_modules`."""
    name: str
    components: [Component] = []
    nets: [Net] = []
    sub_modules?: [Module]

    check:
        len(name) > 0, "module name required"

schema Position:
    """
    Physical placement of a footprint on the board.

    Coordinates in millimetres, rotation in degrees CCW from the
    library orientation. `layer` chooses which side of the board the
    part sits on. Intentionally backend-agnostic — every PCB tool
    speaks "top" and "bottom"; the KiCad backend translates these
    to `F.Cu`/`B.Cu` at emit time.
    """
    x: float
    y: float
    rotation?: float = 0.0
    layer?: "top" | "bottom" = "top"

schema Design:
    """
    The top-level circuit design. One per .k file by convention.

    `placement` is a separate map keyed by reference designator, not
    a field on `Component` — so the *logical* circuit (`root`) is
    decoupled from any one *physical realisation*. A design without
    a `placement` is electrically complete; the converter auto-places
    on a grid.
    """
    name: str
    description?: str
    root: Module
    placement?: {str: Position}

    check:
        len(name) > 0, "design name required"
