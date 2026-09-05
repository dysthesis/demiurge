{
  perSystem = {
    cargoArtifacts,
    commonArgs,
    craneLib,
    ...
  }: {
    checks.demiurge-doc = craneLib.cargoDoc (
      commonArgs
      // {
        inherit cargoArtifacts;
        env.RUSTDOCFLAGS = "--deny warnings";
      }
    );
  };
}
