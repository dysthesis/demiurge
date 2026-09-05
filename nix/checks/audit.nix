{inputs, ...}: {
  perSystem = {
    craneLib,
    src,
    ...
  }: {
    checks.audit = craneLib.cargoAudit {
      inherit src;
      inherit (inputs) advisory-db;
    };
  };
}
