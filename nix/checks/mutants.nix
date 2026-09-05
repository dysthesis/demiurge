{
  perSystem = {
    cargoArtifacts,
    commonArgs,
    craneLib,
    pkgs,
    ...
  }: {
    checks.mutants = craneLib.mkCargoDerivation (
      commonArgs
      // {
        inherit cargoArtifacts;
        pname = "demiurge-mutants";
        nativeBuildInputs = [pkgs.cargo-mutants];
        buildPhaseCargoCommand = "cargo mutants --in-place";
        installPhaseCommand = "mkdir -p $out";
      }
    );
  };
}
