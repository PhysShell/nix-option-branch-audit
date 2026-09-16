{ ... }:
{
  # No `services.synth.foo` assignment visible anywhere in this file --
  # but `imports` means the node's actual config may set it in a file this
  # tool never reads. The old walker just silently found nothing and
  # OBA001'd; must be TestConfigUnresolved instead. The imported path
  # doesn't need to exist -- this tool never follows imports at all, it
  # only needs to recognize the key.
  nodes.machine =
    { ... }:
    {
      imports = [ ./does-not-exist.nix ];
    };
}
