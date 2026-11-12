# Installation

## Prerequisites

- [Nix](https://nixos.org); the flake outputs need the `nix-command` and
  `flakes` features
- a colmena hive as a flake (`colmenaHive` output) or `hive.nix` in a git
  repository, which HIVE deploys

## Flake outputs

| Output                       | Content                                 |
| ---------------------------- | --------------------------------------- |
| `packages.<system>.hive`     | both binaries, `nix run` starts `hive`  |
| `packages.<system>.hive-tui` | same build, `nix run` starts `hive-tui` |
| `packages.<system>.default`  | alias for `hive`                        |
| `overlays.default`           | adds `hive` and `hive-tui` to `pkgs`    |

Supported systems: `x86_64-linux`, `aarch64-linux`.

## Try without installing

```sh
nix run github:farbzen/hive -- --help
nix run github:farbzen/hive#hive-tui
```

## Install on NixOS

```nix
{
  inputs.hive.url = "github:farbzen/hive";

  outputs = {nixpkgs, hive, ...}: {
    nixosConfigurations.<host> = nixpkgs.lib.nixosSystem {
      modules = [
        {
          nixpkgs.overlays = [hive.overlays.default];
        }
        ({pkgs, ...}: {
          environment.systemPackages = [pkgs.hive];
        })
      ];
    };
  };
}
```

> [!NOTE]
> `hive` and `hive-tui` are the same derivation; installing one of them puts
> both binaries on the `PATH`. Both are wrapped with
> [colmena](https://github.com/zhaofengli/colmena) and git, which HIVE calls
> for the deployment and the commits. nix is taken from the NixOS system.

## Install without the flake

[`package.nix`](https://github.com/farbzen/hive/blob/main/package.nix) is a self-contained derivation in nixpkgs
style: it fetches the tagged release with `fetchFromGitHub` and builds it with
the nixpkgs of the importing configuration. Copy it into the configuration,
for example as `hive.nix`, and call it:

```nix
{pkgs, ...}: {
  environment.systemPackages = [(pkgs.callPackage ./hive.nix {})];
}
```

> [!IMPORTANT]
> Copy `package.nix` from `main`, not from a release tag: the hashes of a
> release are committed after its tag.

To switch to another release, change `version` and set `hash` and `cargoHash`
to `""`. The next build fails and prints the correct hashes in its `got:`
lines; paste each in place of its `""`.

> [!NOTE]
> This works without flakes; the build uses the nixpkgs revision of the
> configuration.
