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
