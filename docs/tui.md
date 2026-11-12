# TUI

```sh
hive-tui
```

`hive-tui` is a terminal frontend for HIVE: it lets you pick the options,
nodes and tags in a form, shows the state of the flake repository and runs
HIVE in an embedded terminal, so its prompts keep working.

## Layout

- **Options**: every HIVE option with its flag letter; toggles show `[x]` or
  `[ ]`, text options their value.
- **Update inputs**: the root inputs from the flake's `flake.lock`; selected
  inputs are passed as `-U <input>`. The list is ignored while "Update all
  inputs" (`-u`) is set.
- **Nodes**: the nodes of the hive with their tags; selected nodes are passed
  as `-N <node>`.
- **Tags**: all tags used in the hive with the number of nodes carrying them;
  selected tags are passed as `-T <tag>`.
- **Status**: flake path, targeted nodes, upstream branch, unpushed commits,
  `HEAD` and the uncommitted changes that HIVE will stage and commit.
- **Hints**: combinations HIVE accepts but ignores, such as `-S` without `-P`.
- **Command**: the resulting `hive` command line.

The flake is resolved like HIVE does: the value of the "Flake" option, or the
path saved in `~/.config/hive/flake`.

## Nodes and tags

The node and tag lists come from `colmena eval`, which runs in the background
when the TUI starts, after changing the flake and on reload; the bottom of the
"Nodes" panel shows while it is evaluating or why it failed. "Targets" in the
status shows the nodes the selection reaches: `all` without any selection,
otherwise every selected node plus every node with a selected tag.

## Keys

| Key                     | Action                                        |
| ----------------------- | --------------------------------------------- |
| `j`/`k`, `↓`/`↑`        | move                                          |
| `g`/`G`, `Home`/`End`   | first/last entry                              |
| `Space`, `Enter`        | toggle an option or entry, edit a text option |
| flag letter (`p` … `f`) | select the option and toggle or edit it       |
| `Tab`/`l`               | next list (options, inputs, nodes, tags)      |
| `Shift+Tab`/`h`         | previous list                                 |
| `r`                     | run HIVE                                      |
| `R`                     | reload the status and the hive                |
| `q`, `Esc`, `Ctrl+C`    | quit                                          |

Empty lists are skipped when switching between them.

While editing a text option:

| Key      | Action                       |
| -------- | ---------------------------- |
| `Enter`  | apply                        |
| `Esc`    | cancel and restore the value |
| `Ctrl+U` | clear                        |
| `Ctrl+⌫` | delete the last word         |

## Running HIVE

`r` starts HIVE with the selected options in an embedded terminal. Keys are
passed to HIVE while it runs; `PageUp`/`PageDown` scroll the output. When
HIVE has finished, `q`, `Esc` or `Enter` returns to the form and reloads the
status and the hive.
