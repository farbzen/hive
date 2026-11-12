//! The steps of a HIVE run: prepare, deploy and commit, push.

use anyhow::{Result, bail, ensure};

use crate::{
    cli::Args,
    colmena, confirm,
    flake::{self, Flake},
    git,
};

const DEPLOY_FAILED: &str = "There was an error while deploying! Exiting...";
const COMMIT_FAILED: &str = "There was an error while commiting to the git repo! Exiting...";
const PUSH_FAILED: &str = "There was an error while trying to push! Exiting...";

pub fn run(args: &Args) -> Result<()> {
    let flake = flake::init(args)?;

    prep(args, &flake)?;
    if !args.no_rebuild {
        deploy(args, &flake)?;
    }

    if !args.push || args.dry || args.build_only {
        return Ok(());
    }

    push(args, &flake)
}

/// Pulls, updates inputs and stages all changes (unless the deployment is skipped).
fn prep(args: &Args, flake: &Flake) -> Result<()> {
    ensure!(
        !args.pull || git::pull(&flake.path),
        "There was an error while trying to pull! Exiting..."
    );

    if args.update || !args.update_input.is_empty() {
        ensure!(
            colmena::update_inputs(&flake.path, &args.update_input),
            "There was an error while updating the inputs! Exiting..."
        );
    }

    if !args.no_rebuild {
        ensure!(
            git::stage_all(&flake.path),
            "There was an error while staging all changes! Exiting..."
        );
    }
    Ok(())
}

/// Deploys if anything changed (or on request) and commits the deployment.
fn deploy(args: &Args, flake: &Flake) -> Result<()> {
    let diff = git::staged_diff(&flake.path);

    if diff.is_empty() && !args.pull {
        let _ = cliclack::log::info("No changes found!");

        if args.yes || confirm(false, "Would you like to run the deployment anyway?") {
            ensure!(colmena::deploy(&flake.path, args), DEPLOY_FAILED);
        }
        return Ok(());
    }

    if !diff.is_empty() {
        let _ = cliclack::note("Staged changes", &diff);
    }

    ensure!(colmena::deploy(&flake.path, args), DEPLOY_FAILED);

    if !args.dry && !args.build_only && !diff.is_empty() {
        let nodes = match colmena::nodes(&flake.path) {
            Ok(nodes) => Some(colmena::resolve(&nodes, &args.node, &args.tag)),
            Err(err) => {
                let _ = cliclack::log::warning(format!(
                    "Could not evaluate the deployed nodes ({err}), committing without them..."
                ));
                None
            }
        };
        let mut message = colmena::commit_message(args, nodes.as_deref());
        if let Some(comment) = &args.comment {
            message.push('\n');
            message.push_str(comment);
        }

        ensure!(git::commit(&flake.path, &message), COMMIT_FAILED);

        let _ = cliclack::log::success(format!(
            "Deployment committed!\nThere are currently {} unpushed commits!",
            git::count_unpushed(&flake.path, flake.branch.as_deref())
        ));
    }
    Ok(())
}

/// Pushes unpushed commits, squashing them into one first unless disabled.
fn push(args: &Args, flake: &Flake) -> Result<()> {
    let path = &flake.path;
    let Some(branch) = flake.branch.as_deref() else {
        bail!("There was an error while getting the branch! Exiting...");
    };
    let count = git::count_unpushed(path, Some(branch));

    if count < 1 {
        let _ = cliclack::log::info("Nothing to push! Exiting...");
        return Ok(());
    }

    if count == 1 {
        let _ = cliclack::log::step("Pushing latest commit...");
        ensure!(git::push(path), PUSH_FAILED);
        return Ok(());
    }

    let Some(messages) = git::unpushed(path, branch) else {
        bail!("There was an error while getting unpushed_commits! Exiting...");
    };
    let _ = cliclack::note(format!("{count} new commits found"), messages);

    if args.dont_squash_commits {
        ensure!(git::push(path), PUSH_FAILED);
        return Ok(());
    }

    if !(args.yes
        || confirm(
            true,
            &format!("Would you like to squash the {count} new commits?"),
        ))
    {
        return Ok(());
    }

    ensure!(
        git::staged_diff(path).is_empty(),
        "There are staged changes that would end up in the squashed commit! Exiting..."
    );

    let latest_message = git::latest_message(path);

    ensure!(
        git::squash(path, count),
        "There was an error while squashing commits! Exiting..."
    );

    if !git::commit(path, &latest_message) {
        if git::staged_diff(path).is_empty() {
            let _ = cliclack::log::info(
                "There was no diff after squashing! Seems like you really came full circle...",
            );
            return Ok(());
        }
        bail!(COMMIT_FAILED);
    }

    ensure!(git::push(path), PUSH_FAILED);
    Ok(())
}
