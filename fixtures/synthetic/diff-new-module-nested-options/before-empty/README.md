G1: deliberately empty, mirrors
fixtures/synthetic/audit-diff-module-lifecycle/before-empty -- this
directory exists so `--base-root` canonicalizes, but has no
`module.nix`/`test.nix` at all, simulating a brand-new module file
that does not exist on this side of a real base/head comparison.
