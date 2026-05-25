# KCL package manifest for circuitc.
#
# `name` becomes the root package identifier — `import kcl_pcb.schemas.circuit`
# from anywhere in this tree. `edition` pins the KCL language version so
# schema syntax stays stable across upgrades.

[package]
name = "kcl_pcb"
edition = "v0.11.0"
version = "0.1.0"

[dependencies]
