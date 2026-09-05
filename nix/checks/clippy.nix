{
  perSystem = {
    cargoArtifacts,
    commonArgs,
    craneLib,
    ...
  }: {
    checks.demiurge-clippy = craneLib.cargoClippy (
      commonArgs
      // {
        inherit cargoArtifacts;
        cargoClippyExtraArgs = "--all-targets -- --deny warnings";
      }
    );
  };
}
