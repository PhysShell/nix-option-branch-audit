S2-F2: a real, distinct shape from `before-empty` -- `module.nix` IS
present here (byte-identical to `after-with-module`'s own), but
`test.nix` deliberately is not. Models a real nixpkgs PR that adds a
NixOS test for an already-existing module for the first time (e.g.
`#469112`/`#559239` from S2's own real 30-PR sample) -- the whole
target still becomes `Added` (both module AND test must exist on a
side for that side to count as "present"), but `transition_origin`
must report `analysis_became_possible`, never `subject_added`: the
subject (the module) already existed, only its own analyzability was
new.
