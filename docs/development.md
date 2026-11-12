# Development

## Development environment

```sh
git clone https://github.com/farbzen/hive.git
cd hive
devenv shell              # or: direnv allow
```

[devenv](https://devenv.sh) provides the Nix and Rust tooling; the `.envrc`
loads it automatically with [direnv](https://direnv.net).

## Static analysis and tests

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

The unit tests live in a `tests` module at the end of the source file they
cover and test pure logic only: argument parsing, the node selector and its
resolution, tilde expansion, the commit message, the TUI's command line,
hints, node, tag and input selection, and reading the flake inputs from the
lock file. Calls to git, colmena and nix as well as rendering are not
covered. `nix build` runs the tests as well.

The workflow [`.github/workflows/rust.yml`](https://github.com/farbzen/hive/blob/main/.github/workflows/rust.yml)
runs the same commands in the devenv shell on pushes to `main` and on pull
requests that touch the Cargo workspace, the devenv files or the workflow.
When a GitHub release is published, the workflow runs as well; after lint and
tests have passed, a build job runs `cargo build --workspace --release
--locked` on `x86_64-linux` and `aarch64-linux` and attaches `hive` and
`hive-tui` to the run as the artifacts `hive-x86_64-linux` and
`hive-aarch64-linux`. It also uploads them to the release as
`hive-<system>` and `hive-tui-<system>`.

> [!NOTE]
> The ELF interpreter of the binaries points to the glibc in the Nix store of
> the devenv shell, so they only run where that store path exists, such as
> NixOS with the nixpkgs revision of `devenv.lock`.

## Build and check

```sh
nix build .#hive          # build the workspace
nix flake check           # evaluate all outputs
nix run nixpkgs#alejandra -- flake.nix package.nix
```

> [!IMPORTANT]
> Flakes only see files tracked by git. Run `git add` on new files before
> `nix build` or `nix flake check`.

## Layout

| Path          | Content                                                      |
| ------------- | ------------------------------------------------------------ |
| `Cargo.toml`  | Cargo workspace with the shared version, edition and license |
| `hive/`       | `hive` crate: library and command-line tool                  |
| `hive-tui/`   | `hive-tui` crate: terminal frontend                          |
| `package.nix` | standalone derivation that fetches a release from GitHub     |
| `flake.nix`   | flake outputs; builds `package.nix` from the local checkout  |
| `docs/`       | documentation, also the source of the documentation site     |
| `book.toml`   | [mdBook](https://rust-lang.github.io/mdBook/) configuration  |

The flake overrides `src`, `version` and `cargoDeps` of `package.nix` with the
local workspace, so `hash` and `cargoHash` in `package.nix` only matter for
builds outside the flake.

## Documentation site

The documentation at <https://hive.farbzen.de/> is built with
[mdBook](https://rust-lang.github.io/mdBook/) from the Markdown files in
`docs/`; `book.toml` in the repository root points mdBook at them.
`docs/SUMMARY.md` sets the order of the pages in the sidebar, `docs/README.md`
becomes the start page. Links to files outside `docs/` are absolute GitHub
links, since the book only contains `docs/`.

```sh
mdbook serve --open    # preview on http://localhost:3000
mdbook build           # build into book/
```

New files in `docs/` need an entry in `docs/SUMMARY.md`. The workflow
[`.github/workflows/docs.yml`](https://github.com/farbzen/hive/blob/main/.github/workflows/docs.yml)
builds the book in the devenv shell and deploys it to GitHub Pages on pushes
to `main` that touch `docs/`, `book.toml`, the devenv files or the workflow.
mdBook writes the `CNAME` file for the custom domain `hive.farbzen.de` from
`cname` in `book.toml`; the domain itself is set in the Pages settings of the
repository.

## Release

1. Set the new version in `Cargo.toml`, `package.nix` and the version badge of
   `README.md`, run `cargo check` to update `Cargo.lock`, and move the
   `Unreleased` entries in `CHANGELOG.md` to a new `[<version>] - <date>`
   section with its link at the end of the file.
2. Commit as `chore(release): <version>`, tag the commit as `v<version>` and
   push `main` and the tag.
3. Set `hash` and `cargoHash` in `package.nix` to `""` and build the
   derivation outside the flake:

   ```sh
   nix build --impure --expr 'with import <nixpkgs> {}; callPackage ./package.nix {}'
   ```

   The first build fails on `hash`, the second on `cargoHash`; paste the
   value of each `got:` line in place of its `""`, then build once more.
4. Commit and push the updated `package.nix`.
5. Publish the GitHub release with the changelog section as notes, which
   starts the build job that attaches the binaries:

   ```sh
   gh release create v<version> --verify-tag --title v<version> --notes-file <notes.md>
   ```
