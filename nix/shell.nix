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
            bacon
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
