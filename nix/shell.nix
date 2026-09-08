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
        inherit (config) checks;
        buildInputs = with pkgs; [
          sqlite
        ];
        nativeBuildInputs = with pkgs; [
          pkg-config
        ];
        QLITE3_LIB_DIR = "${pkgs.lib.getLib pkgs.sqlite}/lib";
        SQLITE3_INCLUDE_DIR = "${pkgs.lib.getDev pkgs.sqlite}/include";

        packages =
          (with pkgs; [
            cargo-mutants
            bacon

            sqls
            sqruff

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
