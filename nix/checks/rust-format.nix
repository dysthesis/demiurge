{
  perSystem = {
    craneLib,
    src,
    ...
  }: {
    checks.fmt = craneLib.cargoFmt {inherit src;};
  };
}
