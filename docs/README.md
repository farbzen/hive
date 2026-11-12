# HIVE

**Hive Infrastructure Versioning Engine** deploys a
[colmena](https://github.com/zhaofengli/colmena) hive and records every
deployment as a git commit; it is meant for NixOS users who manage several
machines with a colmena flake in a git repository.

A plain `colmena apply` leaves no trace in the configuration repository. HIVE
wraps update, deployment and commit into one step, so the git history of the
flake doubles as a log of every deployment and the nodes it reached.

- [Installation](installation.md): prerequisites, flake outputs and the
  standalone derivation.
- [Usage](usage.md): options, steps of a run and the commit message.
- [TUI](tui.md): layout and keys of `hive-tui`.
- [Development](development.md): development environment, tests, build,
  documentation and release.

The source code is on [GitHub](https://github.com/farbzen/hive).
