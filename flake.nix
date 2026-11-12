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
      hive = pkgs.rustPlatform.buildRustPackage {
        pname = "hive";
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

        cargoLock.lockFile = ./Cargo.lock;

        nativeBuildInputs = [pkgs.makeWrapper];

        postInstall = ''
          for program in $out/bin/hive $out/bin/hive-tui; do
            wrapProgram "$program" --prefix PATH : ${lib.makeBinPath [pkgs.git pkgs.colmena]}
          done
        '';

        meta = {
          description = "Hive Infrastructure Versioning Engine - Deploying colmena hives and creating commits";
          homepage = "https://github.com/farbzen/hive";
          changelog = "https://github.com/farbzen/hive/blob/v${workspace.workspace.package.version}/CHANGELOG.md";
          license = lib.licenses.mit;
          platforms = lib.platforms.linux;
          mainProgram = "hive";
          maintainers = [
            {
              email = "dev@farbzen.de";
              github = "farbzen";
              githubId = 60570640;
              name = "Fabian Meier";
            }
          ];
        };
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
