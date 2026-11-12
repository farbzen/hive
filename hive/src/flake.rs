//! Locating the flake and the settings HIVE works with.

use std::{fs, path::PathBuf};

use anyhow::{Context, Result};

use crate::{cli::Args, confirm, git};

pub fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// Expands a leading `~` the same way an interactive shell would.
pub fn expand_tilde(path: &str) -> String {
    match (path.strip_prefix('~'), home()) {
        (Some(rest), Some(home)) if rest.is_empty() || rest.starts_with('/') => {
            format!("{}{rest}", home.display())
        }
        _ => path.to_owned(),
    }
}

/// The directory holding HIVE's settings (~/.config/hive).
fn config_dir() -> PathBuf {
    home().unwrap_or_default().join(".config/hive")
}

/// The file holding the saved flake path (~/.config/hive/flake).
pub fn config_file() -> PathBuf {
    config_dir().join("flake")
}

/// The flake path saved in [`config_file`], if any.
pub fn saved_flake() -> Option<String> {
    fs::read_to_string(config_file())
        .ok()
        .map(|text| text.trim().to_owned())
}

pub struct Flake {
    pub path: PathBuf,
    /// The upstream of the current branch, e.g. `origin/main`.
    pub branch: Option<String>,
}

/// Resolves the flake from `--flake` or the saved path, offering to save a
/// new `--flake` value for future runs.
pub fn init(args: &Args) -> Result<Flake> {
    fs::create_dir_all(config_dir())
        .context("There was an error while creating the configuration path!")?;

    let saved = saved_flake().unwrap_or_default();

    let path = match &args.flake {
        Some(flake) => {
            let full = fs::canonicalize(flake)
                .map(|path| path.display().to_string())
                .unwrap_or_default();

            if saved != full
                && confirm(
                    true,
                    "New flake path found! Do you wish to save this path for future use?",
                )
                && fs::write(config_file(), &full).is_err()
            {
                let _ = cliclack::log::error("Error while saving the new flake path! Continuing...");
            }

            PathBuf::from(flake)
        }
        None => fs::canonicalize(&saved).with_context(|| {
            format!(
                "Could not resolve the flake path '{saved}' from {}! Pass one with --flake. Exiting...",
                config_file().display()
            )
        })?,
    };

    Ok(Flake {
        branch: git::upstream(&path),
        path,
    })
}

#[cfg(test)]
mod tests {
    use super::{expand_tilde, home};

    fn home_dir() -> String {
        home().expect("HOME is set").display().to_string()
    }

    #[test]
    fn expands_a_lone_tilde_to_home() {
        assert_eq!(expand_tilde("~"), home_dir());
    }

    #[test]
    fn expands_a_leading_tilde_slash() {
        assert_eq!(expand_tilde("~/nixos"), format!("{}/nixos", home_dir()));
    }

    #[test]
    fn keeps_tilde_user_paths() {
        assert_eq!(expand_tilde("~alice/nixos"), "~alice/nixos");
    }

    #[test]
    fn keeps_paths_without_leading_tilde() {
        assert_eq!(expand_tilde("/etc/nixos"), "/etc/nixos");
        assert_eq!(expand_tilde("nixos/~"), "nixos/~");
        assert_eq!(expand_tilde(""), "");
    }
}
