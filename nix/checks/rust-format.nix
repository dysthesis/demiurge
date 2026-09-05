{
  perSystem = {
    craneLib,
    src,
    ...
  }: {
    checks.demiurge-fmt = craneLib.cargoFmt {inherit src;};
  };
}
