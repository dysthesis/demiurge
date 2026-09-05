{
  perSystem = {
    cargoArtifacts,
    commonArgs,
    craneLib,
    ...
  }: {
    checks.nextest = craneLib.cargoNextest (
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
