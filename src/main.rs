use std::path::PathBuf;

use async_recursion::async_recursion;
use build::BuildError;
use clap::{command, Arg, ArgAction};

mod build;
mod cache;

#[tokio::main]
async fn main() -> Result<(), BuildError> {
    let matches = command!()
        .arg(Arg::new("step").help("Step to run"))
        .arg(
            Arg::new("build_file")
                .short('f')
                .long("file")
                .help("Build file to use"),
        )
        .arg(
            Arg::new("quiet")
                .short('q')
                .long("quiet")
                .help("Quiet mode")
                .action(ArgAction::SetTrue),
        )
        .get_matches();
    let step = matches.get_one::<String>("step").cloned();
    let cache = cache::load_cache(step.clone()).await.unwrap();
    let build_file = matches.get_one::<String>("build_file").cloned();
    let quiet = matches.get_flag("quiet");
    let step = build::get_step(step, &build_file.unwrap_or_else(|| ".".to_string()))?;
    run_step(step, quiet, cache.last_run).await?;
    Ok(())
}

#[async_recursion]
async fn run_step(
    step: build::Step,
    quiet: bool,
    last_run: Option<u64>,
) -> Result<bool, BuildError> {
    let mut handles = vec![];
    // Run dependencies
    for dependency in &step.dependencies {
        if dependency.asynch && !step.in_order {
            handles.push(tokio::spawn(run_step(dependency.clone(), quiet, last_run)));
        } else {
            run_step(dependency.clone(), quiet, last_run).await?;
        }
    }

    let mut ran_steps = vec![];

    for handle in handles {
        ran_steps.push(handle.await??);
    }

    // Check if all dependencies have been skipped
    if !ran_steps.iter().any(|ran_step| *ran_step) {
        // Return false if step is watching files and none of them have changed
        if let Some(file_list) = &step.watch {
            if !watch(file_list.clone(), step.dir.clone(), last_run).await? {
                return Ok(false);
            }
        }
    }

    // Run command
    for command in &step.command {
        let out = || {
            if quiet || step.quiet {
                std::process::Stdio::null()
            } else {
                std::process::Stdio::inherit()
            }
        };
        let dir = step.dir.clone();
        let dir = std::path::Path::new(&dir)
            .canonicalize()?
            .parent()
            .ok_or(BuildError::InvalidPath(dir.clone()))?
            .to_str()
            .ok_or(BuildError::InvalidPath(dir.clone()))?
            .to_string();
        tokio::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(dir)
            .stdout(out())
            .stderr(out())
            .spawn()?
            .wait()
            .await?;
    }
    Ok(true)
}

async fn watch(
    file_list: Vec<String>,
    dir: String,
    last_run: Option<u64>,
) -> Result<bool, BuildError> {
    let dir = if dir.ends_with("build.toml") {
        dir.replace("build.toml", "")
    } else {
        dir
    };
    let Some(last_run) = last_run else {
        return Ok(true);
    };
    for file in file_list {
        let file = std::path::Path::new(&dir)
            .join(file.clone())
            .canonicalize()?;
        let files = get_files_in_dir(file)?;
        for file in files {
            let metadata = std::fs::metadata(file).unwrap();
            let modified = metadata
                .modified()?
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs();

            // Return true if any file has been modified since last run
            if modified > last_run {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn get_files_in_dir(dir: PathBuf) -> Result<Vec<PathBuf>, BuildError> {
    if !dir.is_dir() {
        return Ok(vec![dir]);
    }
    let mut files = vec![];

    for file in dir.read_dir()? {
        let file = file?;
        let file = file.path();
        if file.is_dir() {
            files.append(&mut get_files_in_dir(file)?);
        } else {
            files.push(file);
        }
    }

    Ok(files)
}