{inputs, ...}: {
  perSystem = {
    commonArgs,
    pkgs,
    ...
  }: let
    rustOverlayPkgs = pkgs.appendOverlays [inputs.rust-overlay.overlays.default];

    # Upstream removed MC/DC; pin AdaCore's 2026 reimplementation until it lands.
    stage0 = rustOverlayPkgs.rust-bin.nightly."2026-08-17".minimal;
    mcdcRustSource = pkgs.fetchurl {
      url =
        "https://static.rust-lang.org/dist/"
        + "2026-08-17/rustc-nightly-src.tar.gz";
      hash = "sha256-BA2ezZfzrVqfvfDg/SlMPhBF6kr9e+xdBPT3UHesSSk=";
      passthru.isReleaseTarball = true;
    };
    mcdcPatch = pkgs.fetchurl {
      url =
        "https://github.com/rust-lang/rust/compare/"
        + "165cce8d820b229af8f6a8226cf0b910b57600ff"
        + "...1f916c0b118250311710862a73909887a267ac76.patch";

      hash = "sha256-T2WdI1Z0ypZ7FvcMK+UaskXsVltCHRLKAgswI86z3iU=";
    };

    mcdcRustcUnwrapped = pkgs.rustc.unwrapped.overrideAttrs (old: {
      pname = "rustc-mcdc";
      version = "1.100.0-nightly";
      src = mcdcRustSource;
      patches = (old.patches or []) ++ [mcdcPatch];

      # rustc, coverage runtime, and llvm-cov must share LLVM's profile ABI.
      postPatch =
        old.postPatch
        + ''
          rm -rf src/llvm-project/compiler-rt
          cp -r \
            ${pkgs.llvmPackages.compiler-rt.src}/compiler-rt \
            src/llvm-project/compiler-rt
          chmod -R u+w src/llvm-project/compiler-rt
        '';

      configureFlags =
        map (
          flag:
            if pkgs.lib.hasPrefix "--release-channel=" flag
            then "--release-channel=nightly"
            else if pkgs.lib.hasPrefix "--set=build.rustc=" flag
            then "--set=build.rustc=${stage0}/bin/rustc"
            else if pkgs.lib.hasPrefix "--set=build.cargo=" flag
            then "--set=build.cargo=${stage0}/bin/cargo"
            else flag
        )
        old.configureFlags;
    });

    mcdcRustc = pkgs.rustc.override {
      rustc-unwrapped = mcdcRustcUnwrapped;
    };

    mcdcToolchain = pkgs.symlinkJoin {
      name = "rust-mcdc-toolchain";
      paths = [pkgs.cargo mcdcRustc];
    };

    mcdcCraneLib =
      (inputs.crane.mkLib pkgs).overrideToolchain
      (_: mcdcToolchain);

    coverageArgs =
      commonArgs
      // {
        cargoArtifacts = null;
        LLVM_COV = "${pkgs.llvmPackages.llvm}/bin/llvm-cov";
        LLVM_PROFDATA = "${pkgs.llvmPackages.llvm}/bin/llvm-profdata";
      };

    binaryCoverage = mcdcCraneLib.cargoLlvmCov (
      coverageArgs
      // {
        pname = "demiurge-binary-mcdc";
        cargoLlvmCovCommand = "run";
        cargoLlvmCovExtraArgs = pkgs.lib.concatStringsSep " " [
          "--mcdc"
          "--bin demiurge"
          "--ignore-filename-regex '/src/lib/'"
          "--summary-only"
          "--json"
          "--output-path $out"
        ];
      }
    );

    libraryCoverage = mcdcCraneLib.cargoLlvmCov (
      coverageArgs
      // {
        pname = "demiurge-library-mcdc";
        cargoLlvmCovExtraArgs = pkgs.lib.concatStringsSep " " [
          "--mcdc"
          "--lib"
          "--ignore-filename-regex '/src/main[.]rs$'"
          "--summary-only"
          "--json"
          "--output-path $out"
        ];
      }
    );
  in {
    packages.mcdc-coverage = pkgs.runCommand "demiurge-mcdc-coverage" {} ''
      mkdir -p "$out"
      cp ${binaryCoverage} "$out/binary.json"
      cp ${libraryCoverage} "$out/library.json"
    '';
  };
}
