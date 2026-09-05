{
  perSystem = {pkgs, ...}: let
    inherit (pkgs) lib;
    isLinux = pkgs.stdenv.hostPlatform.isLinux;
    target = pkgs.stdenv.hostPlatform.rust.rustcTarget;
    linkerVariable = "CARGO_TARGET_${lib.toUpper (builtins.replaceStrings ["-"] ["_"] target)}_LINKER";
    linker = pkgs.writeShellScriptBin "demiurge-linker" ''
      output=
      outputFollows=false

      for argument in "$@"; do
        if $outputFollows; then
          output="$argument"
          break
        fi
        if [[ "$argument" == -o ]]; then
          outputFollows=true
        fi
      done

      case "$output" in
        */debug/*)
          exec ${pkgs.stdenv.cc}/bin/cc -B${pkgs.mold}/bin -fuse-ld=mold "$@"
          ;;
        *)
          exec ${pkgs.stdenv.cc}/bin/cc "$@"
          ;;
      esac
    '';
  in {
    _module.args.moldDevelopment = {
      environment = lib.optionalAttrs isLinux {
        "${linkerVariable}" = "${linker}/bin/demiurge-linker";
      };
      packages = lib.optionals isLinux [
        linker
        pkgs.mold
      ];
    };
  };
}
