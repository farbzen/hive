{pkgs, ...}: {
  packages = with pkgs; [
    mdbook
    openssl
    pkg-config
  ];

  languages = {
    rust.enable = true;
    nix.enable = true;
  };
}
