{
  perSystem = {
    cargoArtifacts,
    commonArgs,
    craneLib,
    ...
  }: {
    checks.doc = craneLib.cargoDoc (
      commonArgs
      // {
        inherit cargoArtifacts;
        env.RUSTDOCFLAGS = "--deny warnings";
      }
    );
  };
}
