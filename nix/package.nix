{
  perSystem = {
    cargoArtifacts,
    commonArgs,
    craneLib,
    ...
  }: {
    packages.default = craneLib.buildPackage (
      commonArgs
      // {
        inherit cargoArtifacts;
        meta.mainProgram = "demiurge";
      }
    );
  };
}
