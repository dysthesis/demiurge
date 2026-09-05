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
            nix
            statix
            deadnix
            alejandra
          ])
          ++ moldDevelopment.packages;
      }
      // moldDevelopment.environment
    );
  };
}
