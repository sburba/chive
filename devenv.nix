{
  pkgs,
  lib,
  config,
  ...
}:
{
  packages = [
    pkgs.clang
    pkgs.lld
  ];

  # https://devenv.sh/languages/
  languages.rust = {
    enable = true;
    channel = "stable";

    components = [
      "rustc"
      "cargo"
      "clippy"
      "rustfmt"
      "rust-analyzer"
      "rust-std"
      "rust-src"
    ];
  };
}

