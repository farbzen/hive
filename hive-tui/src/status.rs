//! Information about the flake repository HIVE will operate on.

use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
};

use hive::{
    colmena::{self, Nodes},
    flake::{self, expand_tilde, home},
};

pub struct Status {
    pub flake: Result<PathBuf, String>,
    pub upstream: Option<String>,
    pub unpushed: Option<usize>,
    pub head: Option<String>,
    pub changes: Vec<String>,
    pub inputs: Vec<String>,
}

/// Shortens a path below $HOME to `~/...` for display.
pub fn display_path(path: &Path) -> String {
    match home().and_then(|home| path.strip_prefix(home).ok().map(Path::to_owned)) {
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}

/// Resolves the flake like HIVE: the `--flake` value if given, otherwise
/// the path saved in ~/.config/hive/flake.
fn resolve_flake(flake_field: &str) -> Result<PathBuf, String> {
    let raw = if flake_field.trim().is_empty() {
        flake::saved_flake().ok_or_else(|| {
            format!(
                "no flake configured ({} missing)",
                flake::config_file().display()
            )
        })?
    } else {
        expand_tilde(flake_field.trim())
    };

    std::fs::canonicalize(&raw).map_err(|err| format!("{raw}: {err}"))
}

fn git(path: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .output()
        .ok()?;
    output.status.success().then(|| {
        String::from_utf8_lossy(&output.stdout)
            .trim_end()
            .to_owned()
    })
}

/// Names of the root inputs in the flake's lock file.
fn flake_inputs(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path.join("flake.lock"))
        .map(|text| root_inputs(&text))
        .unwrap_or_default()
}

fn root_inputs(lock_file: &str) -> Vec<String> {
    let Ok(lock) = serde_json::from_str::<serde_json::Value>(lock_file) else {
        return Vec::new();
    };

    let root = lock["root"].as_str().unwrap_or("root");
    let mut inputs: Vec<String> = lock["nodes"][root]["inputs"]
        .as_object()
        .map(|inputs| inputs.keys().cloned().collect())
        .unwrap_or_default();
    inputs.sort();
    inputs
}

/// The nodes of the hive and their tags, evaluated in the background since
/// colmena may have to copy the flake to the store first.
#[derive(Default)]
pub struct Hive {
    pub nodes: Nodes,
    pub error: Option<String>,
    pending: Option<Receiver<Result<Nodes, String>>>,
}

impl Hive {
    /// Starts evaluating the nodes of `flake`, keeping the old ones until done.
    pub fn load(&mut self, flake: Option<&Path>) {
        let Some(path) = flake.map(Path::to_owned) else {
            *self = Self::default();
            return;
        };

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let _ = tx.send(colmena::nodes(&path));
        });
        self.pending = Some(rx);
    }

    /// Picks up the result once the evaluation has finished.
    pub fn poll(&mut self) {
        let Some(rx) = &self.pending else {
            return;
        };
        let result = match rx.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Err("evaluation aborted".to_owned()),
        };
        self.pending = None;
        match result {
            Ok(nodes) => {
                self.nodes = nodes;
                self.error = None;
            }
            Err(err) => {
                self.nodes.clear();
                self.error = Some(err);
            }
        }
    }

    pub fn loading(&self) -> bool {
        self.pending.is_some()
    }

    /// All tags used in the hive, sorted and without duplicates.
    pub fn tags(&self) -> Vec<String> {
        let mut tags: Vec<String> = self.nodes.values().flatten().cloned().collect();
        tags.sort();
        tags.dedup();
        tags
    }
}

pub fn load(flake_field: &str) -> Status {
    let flake = resolve_flake(flake_field);
    let mut status = Status {
        flake: flake.clone(),
        upstream: None,
        unpushed: None,
        head: None,
        changes: Vec::new(),
        inputs: Vec::new(),
    };

    let Ok(path) = flake else {
        return status;
    };

    status.upstream =
        git(&path, &["rev-parse", "--abbrev-ref", "@{upstream}"]).filter(|s| !s.is_empty());
    if let Some(upstream) = &status.upstream {
        status.unpushed = git(
            &path,
            &["rev-list", "--count", &format!("{upstream}..HEAD")],
        )
        .and_then(|count| count.parse().ok());
    }
    status.head = git(&path, &["log", "-1", "--format=%h %s"]);
    status.changes = git(&path, &["status", "--porcelain", "--untracked-files=all"])
        .map(|out| out.lines().map(str::to_owned).collect())
        .unwrap_or_default();
    status.inputs = flake_inputs(&path);
    status
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{Hive, display_path, home, root_inputs};

    #[test]
    fn root_inputs_are_sorted_by_name() {
        let lock = r#"{
            "nodes": {
                "nixpkgs": {},
                "root": {"inputs": {"sops-nix": "sops-nix", "home-manager": "home-manager", "nixpkgs": "nixpkgs"}}
            },
            "root": "root"
        }"#;
        assert_eq!(root_inputs(lock), ["home-manager", "nixpkgs", "sops-nix"]);
    }

    #[test]
    fn root_inputs_follow_a_renamed_root_node() {
        let lock = r#"{
            "nodes": {
                "root": {"inputs": {"ignored": "ignored"}},
                "root_2": {"inputs": {"nixpkgs": "nixpkgs"}}
            },
            "root": "root_2"
        }"#;
        assert_eq!(root_inputs(lock), ["nixpkgs"]);
    }

    #[test]
    fn root_inputs_are_empty_for_invalid_or_input_free_locks() {
        assert!(root_inputs("not json").is_empty());
        assert!(root_inputs(r#"{"nodes": {"root": {}}, "root": "root"}"#).is_empty());
    }

    #[test]
    fn tags_are_collected_from_all_nodes() {
        let mut hive = Hive::default();
        hive.nodes.insert("db1".to_owned(), vec!["core".to_owned()]);
        hive.nodes
            .insert("web1".to_owned(), vec!["web".to_owned(), "core".to_owned()]);
        hive.nodes.insert("web2".to_owned(), Vec::new());
        assert_eq!(hive.tags(), ["core", "web"]);
    }

    #[test]
    fn paths_below_home_are_shortened() {
        let nixos = home().expect("HOME is set").join("nixos");
        assert_eq!(display_path(&nixos), "~/nixos");
    }

    #[test]
    fn paths_outside_home_stay_absolute() {
        assert_eq!(display_path(Path::new("/etc/nixos")), "/etc/nixos");
    }
}
