<div align="center">

# HIVE

_Hive Infrastructure Versioning Engine_

[![Version](https://img.shields.io/badge/version-1.0.0-blue?style=for-the-badge&labelColor=081216)](CHANGELOG.md)
[![Nix Flake](https://img.shields.io/badge/nix-flake-5277C3?style=for-the-badge&labelColor=081216&logo=nixos&logoColor=white)](flake.nix)
[![Rust](https://img.shields.io/badge/rust-2024-orange?style=for-the-badge&labelColor=081216&logo=rust)](Cargo.toml)
[![Docs](https://img.shields.io/badge/docs-github%20pages-0A7EA4?style=for-the-badge&labelColor=081216)](https://hive.farbzen.de/)
[![License: MIT](https://img.shields.io/badge/license-MIT-B0BFC9?style=for-the-badge&labelColor=081216)](LICENSE)
[![Ko-Fi donate](https://img.shields.io/badge/donate-kofi?style=for-the-badge&labelColor=081216&color=ee5556&logo=ko-fi&logoColor=white)](https://ko-fi.com/farbzen)

[Overview](#overview) • [Features](#features) •
[Getting Started](#getting-started) • [Documentation](#documentation)

</div>

HIVE deploys a [colmena](https://github.com/zhaofengli/colmena) hive and
records every deployment as a git commit; it is meant for NixOS users who
manage several machines with a colmena flake in a git repository.

## Overview

A plain `colmena apply` leaves no trace in the configuration repository. HIVE
wraps update, deployment and commit into one step, so the git history of the
flake doubles as a log of every deployment and the nodes it reached. HIVE is
the colmena counterpart of [NARF](https://github.com/farbzen/narf), which does
the same for a single machine with `nh`.

The repository consists of the following applications:

- **hive** – command-line tool that updates, deploys and commits a colmena
  flake, built with [clap](https://docs.rs/clap) and
  [cliclack](https://docs.rs/cliclack). The code lives in `hive/`.
- **hive-tui** – terminal UI frontend for HIVE, built with
  [ratatui](https://ratatui.rs) and [tui-term](https://docs.rs/tui-term). The
  code lives in `hive-tui/`.

## Features

- **Deployment as commit**: every deployment is committed with the node
  selection, the nodes it reached and a timestamp in the message, optionally
  with a comment (`-c`).
- **Node and tag selection**: deploy to selected nodes (`-N`) and tags (`-T`),
  or to the whole hive.
- **Input updates**: update all flake inputs (`-u`) or selected ones (`-U`)
  before deploying.
- **Git sync**: pull before (`-p`) and push after (`-P`) the deployment, with
  unpushed commits squashed into one unless `-S` is given.
- **Safe modes**: dry activation (`-d`), build without deploying (`-b`) and
  pushing existing commits without deploying (`-n`).
- **Scriptable**: answer all prompts with yes (`-y`) and show a trace of
  failing deployments (`-t`).
- **Remembered flake**: the flake path is saved in `~/.config/hive/flake`.
- **TUI**: pick options, nodes and tags in a form or by their flag letter, see
  the repository state and the targeted nodes, and run HIVE in an embedded
  terminal.

## Getting Started

### Prerequisites

- [Nix](https://nixos.org); the flake outputs need the `nix-command` and
  `flakes` features
- a colmena hive as a flake (`colmenaHive` output) or `hive.nix` in a git
  repository, which HIVE deploys

### Installation

Try both applications without installing:

```sh
nix run github:farbzen/hive -- --help
nix run github:farbzen/hive#hive-tui
```

To install them on NixOS, add the flake as an input and its overlay to the
configuration:

```nix
inputs.hive.url = "github:farbzen/hive";
# in a NixOS module:
nixpkgs.overlays = [hive.overlays.default];
environment.systemPackages = [pkgs.hive];
```

> [!NOTE]
> `hive` and `hive-tui` are the same derivation; installing one of them puts
> both binaries on the `PATH`.

Without the flake, copy [`package.nix`](package.nix) into the configuration
and add `(pkgs.callPackage ./hive.nix {})` to `environment.systemPackages`.

The full examples are in [docs/installation.md](docs/installation.md).

### Usage

```sh
hive --update --push      # update inputs, deploy to all nodes, commit and push
hive -T web -N db1        # deploy to all nodes tagged web and to db1
hive --dry                # dry-activate: only show what would change
hive -U nixpkgs -b        # update only nixpkgs, build without deploying
hive-tui                  # interactive frontend
```

Without `--flake`, HIVE reads the flake path from `~/.config/hive/flake` and
offers to save a new `--flake` value there. All options: `hive --help` and
[docs/usage.md](docs/usage.md).

## Documentation

The documentation is published at <https://hive.farbzen.de/>; its
source lives in `docs/`:

- [Installation and flake outputs](docs/installation.md)
- [Usage: options, steps of a run, commit message](docs/usage.md)
- [TUI: layout and keys](docs/tui.md)
- [Development environment, tests, build, documentation site and release](docs/development.md)
- [Changelog](CHANGELOG.md)
- [Legal Notice](docs/legal.md)

## License

See [LICENSE](LICENSE).
