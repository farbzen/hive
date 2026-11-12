{
  description = "HIVE - Hive Infrastructure Versioning Engine";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = {
    self,
    nixpkgs,
  }: let
    inherit (nixpkgs) lib;
    forAllSystems = lib.genAttrs ["x86_64-linux" "aarch64-linux"];
    workspace = lib.importTOML ./Cargo.toml;
  in {
    packages = forAllSystems (system: let
      pkgs = nixpkgs.legacyPackages.${system};
      hive = (pkgs.callPackage ./package.nix {}).overrideAttrs {
        inherit (workspace.workspace.package) version;
        src = lib.fileset.toSource {
          root = ./.;
          fileset = lib.fileset.unions [
            ./Cargo.toml
            ./Cargo.lock
            ./hive/Cargo.toml
            ./hive/src
            ./hive-tui/Cargo.toml
            ./hive-tui/src
          ];
        };
        cargoDeps = pkgs.rustPlatform.importCargoLock {lockFile = ./Cargo.lock;};
      };
    in {
      inherit hive;
      default = hive;
      hive-tui = hive // {meta = hive.meta // {mainProgram = "hive-tui";};};
    });

    overlays.default = final: _prev: {
      inherit (self.packages.${final.stdenv.hostPlatform.system}) hive hive-tui;
    };
  };
}
