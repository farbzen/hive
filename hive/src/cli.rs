//! Command line arguments.

use clap::Parser;

/// HIVE (Hive Infrastructure Versioning Engine) - Deploying colmena hives and creating commits
#[derive(Parser, Debug)]
#[command(name = "hive", version)]
pub struct Args {
    /// Build the configurations without deploying them
    #[arg(short, long)]
    pub build_only: bool,

    /// Add a comment to the resulting commit
    #[arg(short, long)]
    pub comment: Option<String>,

    /// Run in 'dry' mode. Deploys with the 'dry-activate' goal, which only states what WOULD change. No commit, no push
    #[arg(short, long)]
    pub dry: bool,

    /// Path to the flake (must be a git repository). Checks configuration in ".config/hive/flake" by default
    #[arg(short, long)]
    pub flake: Option<String>,

    /// Deploy to this node (globs are supported). Can be used multiple times. Deploys to all nodes if neither nodes nor tags are given
    #[arg(short = 'N', long)]
    pub node: Vec<String>,

    /// Skip the deployment and only push (and squash) existing commits. Uncommitted changes are left untouched
    #[arg(
        short = 'n',
        long,
        conflicts_with_all = ["update", "update_input", "comment", "build_only", "trace"]
    )]
    pub no_rebuild: bool,

    /// Pull new commits from origin before running
    #[arg(short, long)]
    pub pull: bool,

    /// Push new commits to origin
    #[arg(short = 'P', long)]
    pub push: bool,

    /// Don't squash commits before pushing to origin (only applicable when combined with -P/--push)
    #[arg(short = 'S', long)]
    pub dont_squash_commits: bool,

    /// Deploy to all nodes with this tag. Can be used multiple times
    #[arg(short = 'T', long)]
    pub tag: Vec<String>,

    /// Show trace of the deployment (useful if the evaluation keeps failing)
    #[arg(short, long)]
    pub trace: bool,

    /// Update all inputs before running
    #[arg(short, long)]
    pub update: bool,

    /// Update a specific input before running. Can be used multiple times to specify more than one specific input
    #[arg(short = 'U', long)]
    pub update_input: Vec<String>,

    /// Say yes to all (Non-interactive)
    #[arg(short, long)]
    pub yes: bool,
}

impl Args {
    /// The colmena node selector (`--on`): the nodes, then the tags prefixed
    /// with `@`; `None` deploys to all nodes.
    pub fn on(&self) -> Option<String> {
        let selectors: Vec<String> = self
            .node
            .iter()
            .cloned()
            .chain(self.tag.iter().map(|tag| format!("@{tag}")))
            .collect();
        (!selectors.is_empty()).then(|| selectors.join(","))
    }
}

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, Parser};

    use super::Args;

    fn parse(args: &[&str]) -> Args {
        Args::try_parse_from(std::iter::once("hive").chain(args.iter().copied()))
            .expect("valid arguments")
    }

    #[test]
    fn command_definition_is_valid() {
        Args::command().debug_assert();
    }

    #[test]
    fn everything_is_off_without_arguments() {
        let args = parse(&[]);
        assert!(!args.build_only && !args.dry && !args.no_rebuild);
        assert!(!args.pull && !args.push && !args.dont_squash_commits);
        assert!(!args.trace && !args.update && !args.yes);
        assert!(args.comment.is_none() && args.flake.is_none());
        assert!(args.update_input.is_empty() && args.node.is_empty() && args.tag.is_empty());
        assert_eq!(args.on(), None);
    }

    #[test]
    fn combined_short_flags_set_every_switch() {
        let args = parse(&["-bdptuyPS"]);
        assert!(args.build_only && args.dry);
        assert!(args.pull && args.push && args.dont_squash_commits);
        assert!(args.trace && args.update && args.yes);
    }

    #[test]
    fn no_rebuild_combines_with_pull_and_push() {
        let args = parse(&["-npPS"]);
        assert!(args.no_rebuild && args.pull && args.push && args.dont_squash_commits);
    }

    #[test]
    fn no_rebuild_rejects_rebuild_options() {
        for conflicting in [
            &["-u"][..],
            &["-U", "nixpkgs"],
            &["-c", "comment"],
            &["-b"],
            &["-t"],
        ] {
            let args = ["hive", "-n"]
                .into_iter()
                .chain(conflicting.iter().copied());
            assert!(Args::try_parse_from(args).is_err(), "{conflicting:?}");
        }
    }

    #[test]
    fn values_are_read_from_short_and_long_options() {
        let short = parse(&["-c", "new kernel", "-f", "/etc/nixos"]);
        let long = parse(&["--comment", "new kernel", "--flake", "/etc/nixos"]);
        for args in [short, long] {
            assert_eq!(args.comment.as_deref(), Some("new kernel"));
            assert_eq!(args.flake.as_deref(), Some("/etc/nixos"));
        }
    }

    #[test]
    fn update_input_can_be_repeated() {
        let args = parse(&["-U", "nixpkgs", "--update-input", "home-manager"]);
        assert_eq!(args.update_input, ["nixpkgs", "home-manager"]);
    }

    #[test]
    fn nodes_and_tags_can_be_repeated() {
        let args = parse(&[
            "-N", "db1", "--node", "web-*", "-T", "edge", "--tag", "core",
        ]);
        assert_eq!(args.node, ["db1", "web-*"]);
        assert_eq!(args.tag, ["edge", "core"]);
    }

    #[test]
    fn selector_lists_nodes_then_tags() {
        let args = parse(&["-T", "web", "-N", "db1", "-T", "core"]);
        assert_eq!(args.on().as_deref(), Some("db1,@web,@core"));
    }

    #[test]
    fn selector_works_with_tags_only() {
        assert_eq!(parse(&["-T", "web"]).on().as_deref(), Some("@web"));
    }

    #[test]
    fn unknown_flags_are_rejected() {
        assert!(Args::try_parse_from(["hive", "--unknown"]).is_err());
        assert!(Args::try_parse_from(["hive", "-H", "laptop"]).is_err());
    }

    #[test]
    fn options_require_a_value() {
        assert!(Args::try_parse_from(["hive", "--node"]).is_err());
    }
}
