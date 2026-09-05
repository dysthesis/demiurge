{
  perSystem = {
    craneLib,
    src,
    ...
  }: {
    checks.deny = craneLib.cargoDeny {inherit src;};
  };
}
