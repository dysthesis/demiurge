{
  perSystem = {
    config,
    craneLib,
    moldDevelopment,
    pkgs,
    ...
  }: {
    devShells.default = craneLib.devShell (
      {
        checks = config.checks;
        packages =
          (with pkgs; [
            cargo-mutants
            statix
            deadnix
            alejandra
            jq
          ])
          ++ moldDevelopment.packages;
      }
      // moldDevelopment.environment
    );
  };
}
