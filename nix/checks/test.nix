{
  perSystem = {
    cargoArtifacts,
    commonArgs,
    craneLib,
    ...
  }: {
    checks.demiurge-nextest = craneLib.cargoNextest (
      commonArgs
      // {
        inherit cargoArtifacts;
        partitions = 1;
        partitionType = "count";
        cargoNextestPartitionsExtraArgs = "--no-tests=pass";
      }
    );
  };
}
