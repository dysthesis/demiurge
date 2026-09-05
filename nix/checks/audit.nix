{inputs, ...}: {
  perSystem = {
    craneLib,
    src,
    ...
  }: {
    checks.demiurge-audit = craneLib.cargoAudit {
      inherit src;
      advisory-db = inputs.advisory-db;
    };
  };
}
