//! Calls to colmena and nix.

use std::{
    collections::BTreeMap,
    path::Path,
    process::{Command, Stdio},
};

use crate::{cli::Args, succeeds};

/// The tags of every node in the hive, keyed by node name.
pub type Nodes = BTreeMap<String, Vec<String>>;

const NODE_TAGS_EXPRESSION: &str =
    "{ nodes, ... }: builtins.mapAttrs (_: node: node.config.deployment.tags) nodes";

/// The colmena goal of a run: `build`, `dry-activate` or `switch`.
pub fn goal(args: &Args) -> &'static str {
    if args.build_only {
        "build"
    } else if args.dry {
        "dry-activate"
    } else {
        "switch"
    }
}

fn colmena(path: &Path) -> Command {
    let mut cmd = Command::new("colmena");
    cmd.current_dir(path);
    cmd
}

pub fn deploy(path: &Path, args: &Args) -> bool {
    let mut cmd = colmena(path);
    cmd.args(["apply", goal(args)])
        .args(["--nix-option", "accept-flake-config", "true"]);

    if let Some(on) = args.on() {
        cmd.args(["--on", &on]);
    }
    if args.trace {
        cmd.arg("--show-trace");
    }

    succeeds(&mut cmd)
}

/// Evaluates the hive for its nodes and their tags.
pub fn nodes(path: &Path) -> Result<Nodes, String> {
    let output = colmena(path)
        .args(["eval", "-E", NODE_TAGS_EXPRESSION])
        .args(["--nix-option", "accept-flake-config", "true"])
        .stdin(Stdio::null())
        .output()
        .map_err(|err| format!("could not run colmena: {err}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let reason = stderr
            .lines()
            .rev()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("colmena eval failed");
        return Err(reason.trim().to_owned());
    }

    serde_json::from_slice(&output.stdout)
        .map_err(|err| format!("unexpected colmena output: {err}"))
}

/// Matches a node name against a colmena glob, where `*` stands for any text.
fn glob_matches(pattern: &str, name: &str) -> bool {
    match pattern.split_once('*') {
        None => pattern == name,
        Some((prefix, rest)) => {
            let Some(name) = name.strip_prefix(prefix) else {
                return false;
            };
            (0..=name.len())
                .filter(|&i| name.is_char_boundary(i))
                .any(|i| glob_matches(rest, &name[i..]))
        }
    }
}

/// The nodes a deployment reaches: those matching one of `node_patterns` or
/// carrying one of `tag_patterns`; all nodes if both are empty.
pub fn resolve(nodes: &Nodes, node_patterns: &[String], tag_patterns: &[String]) -> Vec<String> {
    let select_all = node_patterns.is_empty() && tag_patterns.is_empty();
    nodes
        .iter()
        .filter(|(name, tags)| {
            select_all
                || node_patterns
                    .iter()
                    .any(|pattern| glob_matches(pattern, name))
                || tag_patterns
                    .iter()
                    .any(|pattern| tags.iter().any(|tag| glob_matches(pattern, tag)))
        })
        .map(|(name, _)| name.clone())
        .collect()
}

/// Updates the given inputs, or all of them if `inputs` is empty.
pub fn update_inputs(path: &Path, inputs: &[String]) -> bool {
    succeeds(
        Command::new("nix")
            .args(["flake", "update", "--flake"])
            .arg(path)
            .args(inputs),
    )
}

fn now() -> String {
    Command::new("date")
        .arg("+%Y-%m-%dT%H:%M:%S")
        .stderr(Stdio::null())
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .unwrap_or_default()
}

/// Describes the deployment, e.g.
/// `deploy(@web,db1): switch\n\nNodes: db1, web1\n\nDeployed: 2026-11-12T10:15:00`.
pub fn commit_message(args: &Args, nodes: Option<&[String]>) -> String {
    describe_deployment(args.on().as_deref(), goal(args), nodes, &now())
}

fn describe_deployment(
    on: Option<&str>,
    goal: &str,
    nodes: Option<&[String]>,
    deployed: &str,
) -> String {
    let mut message = format!("deploy({}): {goal}\n\n", on.unwrap_or("all"));
    if let Some(nodes) = nodes {
        message.push_str(&format!("Nodes: {}\n\n", nodes.join(", ")));
    }
    message.push_str(&format!("Deployed: {deployed}"));
    message
}

#[cfg(test)]
mod tests {
    use super::{Nodes, describe_deployment, glob_matches, resolve};

    fn hive() -> Nodes {
        [
            ("db1", vec!["core"]),
            ("edge-a", vec!["edge", "web"]),
            ("edge-b", vec!["edge"]),
            ("web1", vec!["web"]),
        ]
        .into_iter()
        .map(|(name, tags)| {
            (
                name.to_owned(),
                tags.into_iter().map(str::to_owned).collect(),
            )
        })
        .collect()
    }

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn globs_match_any_text_for_a_star() {
        assert!(glob_matches("edge-*", "edge-a"));
        assert!(glob_matches("*-a", "edge-a"));
        assert!(glob_matches("e*e-*", "edge-a"));
        assert!(glob_matches("*", ""));
        assert!(!glob_matches("edge-*", "web1"));
        assert!(!glob_matches("edge", "edge-a"));
    }

    #[test]
    fn empty_selection_reaches_every_node() {
        assert_eq!(
            resolve(&hive(), &[], &[]),
            ["db1", "edge-a", "edge-b", "web1"]
        );
    }

    #[test]
    fn nodes_and_tags_are_combined() {
        assert_eq!(
            resolve(&hive(), &strings(&["db1"]), &strings(&["web"])),
            ["db1", "edge-a", "web1"]
        );
    }

    #[test]
    fn node_and_tag_globs_are_resolved() {
        assert_eq!(
            resolve(&hive(), &strings(&["edge-*"]), &[]),
            ["edge-a", "edge-b"]
        );
        assert_eq!(resolve(&hive(), &[], &strings(&["co*"])), ["db1"]);
    }

    #[test]
    fn unknown_selections_reach_no_node() {
        assert!(resolve(&hive(), &strings(&["mail"]), &[]).is_empty());
    }

    #[test]
    fn describes_the_deployment_with_its_nodes() {
        assert_eq!(
            describe_deployment(
                Some("db1,@web"),
                "switch",
                Some(&strings(&["db1", "web1"])),
                "2026-11-12T10:15:00"
            ),
            "deploy(db1,@web): switch\n\n\
             Nodes: db1, web1\n\n\
             Deployed: 2026-11-12T10:15:00"
        );
    }

    #[test]
    fn describes_a_deployment_to_all_nodes_without_known_nodes() {
        assert_eq!(
            describe_deployment(None, "switch", None, "2026-11-12T10:15:00"),
            "deploy(all): switch\n\nDeployed: 2026-11-12T10:15:00"
        );
    }
}
