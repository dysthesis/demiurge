{inputs, ...}: {
  perSystem = {pkgs, ...}: let
    craneLib = inputs.crane.mkLib pkgs;
    src = craneLib.cleanCargoSource ../.;
    commonArgs = {
      inherit src;
      strictDeps = true;
      buildInputs = pkgs.lib.optionals pkgs.stdenv.hostPlatform.isDarwin [
        pkgs.libiconv
      ];
    };
    cargoArtifacts = craneLib.buildDepsOnly commonArgs;
  in {
    _module.args = {
      inherit cargoArtifacts commonArgs craneLib src;
    };
  };
}
