S1-F1: deliberately empty on purpose -- this directory exists (so
`--base-root`/`--head-root` itself canonicalizes fine) but contains no
`module.nix`/`test.nix` at all, simulating a target manifest naming a
module that doesn't exist yet on this side of a real base/head
comparison (a brand-new NixOS service module -- see S1's own real
`kener`/`yace` TOOL_ERROR finding). Only this README lives here; that's
the point.
