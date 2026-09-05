{
  perSystem = {
    craneLib,
    pkgs,
    src,
    ...
  }: {
    checks.demiurge-toml-fmt = craneLib.taploFmt {
      src = pkgs.lib.sources.sourceFilesBySuffices src [".toml"];
    };
  };
}
