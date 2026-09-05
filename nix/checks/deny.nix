{
  perSystem = {
    craneLib,
    src,
    ...
  }: {
    checks.demiurge-deny = craneLib.cargoDeny {inherit src;};
  };
}
