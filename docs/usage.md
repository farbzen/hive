# Usage

```sh
hive [OPTIONS]
```

HIVE works on a [colmena](https://github.com/zhaofengli/colmena) hive in a git
repository. Each run updates the flake inputs on request, deploys the hive
with `colmena apply`, commits the deployment and optionally pushes it.

## Options

| Short | Long                    | Value | Effect                                                                 |
| ----- | ----------------------- | ----- | ---------------------------------------------------------------------- |
| `-b`  | `--build-only`          |       | build the configurations without deploying them (`colmena build`)      |
| `-c`  | `--comment`             | text  | add a comment to the resulting commit                                  |
| `-d`  | `--dry`                 |       | deploy with the `dry-activate` goal: only state what would change      |
| `-f`  | `--flake`               | path  | path to the flake (a git repository); defaults to the saved path       |
| `-N`  | `--node`                | node  | deploy to this node (globs supported); repeat for more nodes           |
| `-n`  | `--no-rebuild`          |       | skip the deployment and only push (and squash) existing commits        |
| `-p`  | `--pull`                |       | pull new commits from origin before running                            |
| `-P`  | `--push`                |       | push new commits to origin                                             |
| `-S`  | `--dont-squash-commits` |       | don't squash unpushed commits before pushing (only with `-P`)          |
| `-T`  | `--tag`                 | tag   | deploy to all nodes with this tag (globs supported); repeat for more   |
| `-t`  | `--trace`               |       | show a trace of the deployment                                         |
| `-u`  | `--update`              |       | update all flake inputs before running                                 |
| `-U`  | `--update-input`        | input | update a specific input before running; repeat for more inputs         |
| `-y`  | `--yes`                 |       | answer all prompts with yes (non-interactive)                          |
| `-h`  | `--help`                |       | print help                                                             |
| `-V`  | `--version`             |       | print version                                                          |

Short flags can be combined, e.g. `hive -upP`.

> [!NOTE]
> `-n` can't be combined with `-u`, `-U`, `-c`, `-b` or `-t`. Uncommitted
> changes are left untouched in this mode.

## Node selection

`-N` and `-T` are combined into colmena's node selector `--on`: the nodes
first, then the tags prefixed with `@`. `hive -N db1 -T web -T edge` deploys
with `--on db1,@web,@edge`, i.e. to `db1` and to every node tagged `web` or
`edge`. Without `-N` and `-T`, HIVE deploys to all nodes of the hive.

## Flake path

Without `--flake`, HIVE reads the flake path from `~/.config/hive/flake`. When
`--flake` points to a different path than the saved one, HIVE offers to save
it for future runs. colmena runs inside this directory and picks up its
`flake.nix` or `hive.nix`.

## Run

A run consists of these steps:

1. **Pull** (`-p`): `git pull` in the flake repository.
2. **Update** (`-u`, `-U`): `nix flake update` for all or the given inputs.
3. **Stage**: `git add` of all changes in the repository (skipped with `-n`).
4. **Deploy**: `colmena apply switch` (`build` with `-b`, `dry-activate` with
   `-d`) for the selected nodes, with `--show-trace` for `-t`. Without staged
   changes and without `-p`, HIVE asks whether to deploy anyway (`-y` answers
   yes).
5. **Commit**: if there were staged changes and neither `-d` nor `-b` is set,
   the deployment is committed.
6. **Push** (`-P`, skipped with `-d` and `-b`): a single unpushed commit is
   pushed as it is. Several unpushed commits are squashed into one with the
   message of the latest commit, after a confirmation, unless `-S` is given.

> [!IMPORTANT]
> Pushing needs an upstream branch. Squashing is refused while there are
> staged changes, since they would end up in the squashed commit.

## Commit message

The message names the node selection (`all` without `-N`/`-T`) and the goal,
lists the nodes the selection reached and the time of the deployment; a
comment from `-c` is appended as its last line:

```text
deploy(db1,@web): switch

Nodes: db1, web1, web2

Deployed: 2026-11-12T10:15:00
<comment>
```

The nodes are taken from `colmena eval`, with globs resolved like colmena
does. If the evaluation fails, the `Nodes:` line is left out.

## Exit codes

| Code | Meaning           |
| ---- | ----------------- |
| `0`  | success           |
| `1`  | a step failed     |
| `2`  | invalid arguments |

## Examples

```sh
hive --update --push            # update inputs, deploy to all nodes, commit and push
hive -T web -N db1              # deploy to the nodes tagged web and to db1
hive -N 'edge-*' --dry          # dry-activate all edge nodes
hive -U nixpkgs -b              # update only nixpkgs, build without deploying
hive -c "new kernel" -P         # deploy, commit with a comment and push
hive -n -P                      # squash and push existing deployment commits
hive -f ~/infra -T core -y      # deploy another hive's core nodes without prompts
```
