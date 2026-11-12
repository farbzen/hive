# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0] - 2026-11-12

### Added

- `hive` command-line tool, derived from
  [NARF](https://github.com/farbzen/narf) 2.1.0, that deploys a colmena hive
  via `colmena apply` and commits every deployment with the node selection,
  the nodes it reached and a timestamp.
- Selecting the target nodes by name (`-N`, globs supported) and by tag
  (`-T`); both are combined into colmena's `--on`, without them the whole hive
  is deployed.
- Updating all flake inputs (`-u`) or selected ones (`-U`) before a
  deployment.
- Pulling before (`-p`) and pushing after (`-P`) a deployment, squashing
  unpushed commits unless `-S` is given.
- Dry activation (`-d`, colmena goal `dry-activate`), build without deploying
  (`-b`, goal `build`), pushing existing commits without deploying (`-n`),
  commit comments (`-c`), trace output (`-t`) and non-interactive mode (`-y`).
- Saved flake path in `~/.config/hive/flake`.
- `hive-tui` terminal frontend with an options form, lists of the hive's nodes
  and tags (evaluated in the background), the targeted nodes in the status,
  the repository status (upstream, unpushed commits, changes, inputs) and HIVE
  running in an embedded terminal.
- Nix flake with `hive`, `hive-tui` and `default` packages and an overlay for
  `x86_64-linux` and `aarch64-linux`; `hive` and `hive-tui` bring `colmena`
  and `git` on their `PATH`.
- MIT license, declared in the Cargo manifests and in the `meta.license` of
  the Nix packages.
- Standalone `package.nix` that fetches a tagged release via
  `fetchFromGitHub`.
- Unit tests for argument parsing, the node selector and its resolution, tilde
  expansion, the commit message, the TUI's command line, hints, node, tag and
  input selection, and reading the flake inputs from the lock file.

[Unreleased]: https://github.com/farbzen/hive/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/farbzen/hive/releases/tag/v1.0.0
