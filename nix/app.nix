{
  perSystem = {
    config,
    pkgs,
    ...
  }: {
    apps.default = {
      type = "app";
      program = pkgs.lib.getExe config.packages.default;
      meta.description = "Run demiurge";
    };
  };
}
