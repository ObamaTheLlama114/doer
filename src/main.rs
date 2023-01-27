use std::io::Error;

use async_recursion::async_recursion;
use clap::{command, Arg, ArgAction};

mod build;

#[tokio::main]
async fn main() {
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
    let build_file = matches.get_one::<String>("build_file").cloned();
    let quiet = matches.get_flag("quiet");
    let step = build::get_step(step, &build_file.unwrap_or_else(|| ".".to_string()));
    match step {
        Ok(step) => run_step(step, quiet).await.unwrap(),
        Err(error) => {
            match error {
                build::BuildError::IoError(error) => println!("{}", error),
                build::BuildError::TomlError(error) => println!("{}", error),
                build::BuildError::MissingStep(step) => println!("Step not found: {}", step),
                build::BuildError::InvalidPath(path) => println!("Invalid build.toml: {}", path),
                build::BuildError::InvalidStep(step) => println!("Invalid step name: {}", step),
            };
            std::process::exit(1)
        }
    };
}

#[async_recursion]
async fn run_step(step: build::Step, quiet: bool) -> Result<(), Error> {
    let mut handles = vec![];
    // Run dependencies
    for dependency in &step.dependencies {
        if dependency.asynch && !step.in_order {
            handles.push(tokio::spawn(run_step(dependency.clone(), quiet)));
        } else {
            run_step(dependency.clone(), quiet).await?;
        }
    }

    // Wait for async dependencies
    for handle in handles {
        let result = handle.await;
        if let Err(error) = &result {
            println!("Error: {}", error);
            std::process::exit(1);
        }
        if let Ok(result) = result {
            if let Err(error) = result {
                println!("Error: {}", error);
                std::process::exit(1);
            }
        }
    }

    // Run command
    if let Some(command) = &step.command {
        let dir = step.dir.clone();
        let dir = std::path::Path::new(&dir)
            .canonicalize()
            .expect("Could not canonicalize path")
            .parent()
            .expect("Could not get parent")
            .to_str()
            .expect("Could not convert path to string")
            .to_string();
        tokio::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(dir)
            .stdout(if quiet {
                std::process::Stdio::null()
            } else {
                std::process::Stdio::inherit()
            })
            .stderr(if quiet {
                std::process::Stdio::null()
            } else {
                std::process::Stdio::inherit()
            })
            .spawn()?
            .wait()
            .await?;
    }
    Ok(())
}
