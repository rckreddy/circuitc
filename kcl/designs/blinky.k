"""
Blinky: the hardware "hello world".

A 3.3 V supply through a current-limiting resistor lights an LED to ground.

    VCC (3.3 V) ──[ R1: 1 kΩ ]──┬── (LED anode)
                                │
                                D1  (LED_RED)
                                │
                                └── GND

This file is a *design* — an instance of the schemas in
`../schemas/circuit.k`. It declares the components, declares the nets
that connect them, and bundles both into a single top-level `Design`
that KCL emits as JSON for the Rust converter to consume.
"""

import ..schemas.circuit as c

# === Components ===
# Two parts: a current-limiting resistor and an LED.

r1 = c.Component {
    reference = "R1"
    value = "1kohm"
    footprint = {library = "Resistor_SMD", name = "R_0402_1005Metric"}
    pins = [
        {name = "1", number = 1, direction = "passive"},
        {name = "2", number = 2, direction = "passive"},
    ]
    manufacturer = "Yageo"
    mpn = "RC0402FR-071KL"
}

d1 = c.Component {
    reference = "D1"
    value = "LED_RED"
    footprint = {library = "LED_SMD", name = "LED_0402_1005Metric"}
    pins = [
        {name = "A", number = 1, direction = "passive"},  # Anode
        {name = "K", number = 2, direction = "passive"},  # Cathode
    ]
    manufacturer = "Wurth"
    mpn = "150040RS75000"
}

# === Nets ===
# Three rails: VCC into R1, the LED_ANODE node between R1 and D1, GND
# out of D1's cathode.

vcc = c.Net {
    name = "VCC"
    kind = "power"
    voltage = 3.3
    connections = [
        {component = "R1", pin = 1},
    ]
}

led_anode = c.Net {
    name = "LED_ANODE"
    kind = "signal"
    connections = [
        {component = "R1", pin = 2},
        {component = "D1", pin = 1},
    ]
}

gnd = c.Net {
    name = "GND"
    kind = "ground"
    voltage = 0.0
    connections = [
        {component = "D1", pin = 2},
    ]
}

# === Top-level design ===
# `design` is the symbol the converter looks for in the JSON output —
# `KclOutput { design: Design }` on the Rust side.
#
# `placement` is optional. Omit it and the converter auto-places
# components on a grid. Here we set explicit positions so KiCad opens
# the .kicad_pcb file with R1 and D1 already 5 mm apart, ratsnest
# tidy, no manual spreading needed.

design = c.Design {
    name = "blinky"
    description = "Single-LED indicator powered from 3.3 V"
    root = c.Module {
        name = "blinky"
        components = [r1, d1]
        nets = [vcc, led_anode, gnd]
    }
    placement = {
        "R1" = c.Position {x = 0.0, y = 0.0}
        "D1" = c.Position {x = 5.0, y = 0.0}
    }
}
