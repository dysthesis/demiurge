{
  perSystem = {pkgs, ...}: {
    devShells.default = pkgs.mkShellNoCC {
      name = "demiurge-dev";
      packages = with pkgs; [
        # Nix tooling
        nix
        statix
        deadnix
        alejandra
      ];
    };
  };
}
