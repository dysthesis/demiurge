{
  perSystem = {
    config,
    pkgs,
    ...
  }: {
    checks.binary-mcdc =
      pkgs.runCommand "demiurge-binary-mcdc" {
        nativeBuildInputs = [pkgs.jq];
      } ''
        jq -e '
          .data as $data
          | ($data | length == 1)
            and ($data[0].files | length > 0)
            and any($data[0].files[]; .filename | endswith("/src/main.rs"))
            and all($data[0].files[]; .filename | contains("/src/lib/") | not)
            # LLVM reports 0% for 0/0; zero uncovered obligations is vacuous 100%.
            and ($data[0].totals.mcdc | .covered == .count and .notcovered == 0)
        ' ${config.packages.mcdc-coverage}/binary.json >/dev/null
        touch "$out"
      '';
  };
}
