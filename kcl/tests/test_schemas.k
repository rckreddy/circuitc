"""
Smoke tests for the circuit schemas.

Run with `just test-kcl` (which calls `kcl test ./tests/...`). Each
`test_*` lambda is picked up automatically by KCL's test runner.

Negative cases are kept as commented-out instantiations rather than
runtime assertions: a schema check failure is a *compile-time* error
in KCL, so the only way to verify a check fires is to uncomment the
offending block and confirm `kcl run` rejects it.
"""

import ..schemas.circuit as c

test_valid_resistor = lambda {
    r = c.Component {
        reference = "R1"
        value = "10kohm"
        footprint = {library = "Resistor_SMD", name = "R_0402_1005Metric"}
        pins = [
            {name = "1", number = 1},
            {name = "2", number = 2},
        ]
    }
    assert r.reference == "R1"
    assert len(r.pins) == 2
}

test_valid_net = lambda {
    n = c.Net {
        name = "VCC"
        kind = "power"
        voltage = 3.3
        connections = [
            {component = "R1", pin = 1},
        ]
    }
    assert n.name == "VCC"
    assert len(n.connections) == 1
}

# Negative cases — uncomment one at a time to confirm the check fires.
#
# bad_reference = c.Component {
#     reference = "r1"               # lowercase fails the regex check
#     value = "foo"
#     footprint = {library = "x", name = "y"}
#     pins = [{name = "1", number = 1}]
# }
#
# empty_net = c.Net {
#     name = "FLOATING"
#     connections = []                # fails: nets need ≥1 connection
# }
