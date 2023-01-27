use std::io::Error;

use async_recursion::async_recursion;
use clap::Parser;

mod build;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to build.toml
    #[arg(short = 'f', long = "file")]
    build_file: Option<String>,

    /// Command to run
    #[arg(short = 's', long = "step")]
    step: Option<String>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let step = build::get_step(
        args.step,
        &args.build_file.unwrap_or_else(|| ".".to_string()),
    );
    match step {
        Ok(step) => run_step(step).await.unwrap(),
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
async fn run_step(step: build::Step) -> Result<(), Error> {
    let mut handles = vec![];
    // Run dependencies
    for dependency in &step.dependencies {
        if dependency.asynch && !step.in_order {
            handles.push(run_step(dependency.clone()));
        } else {
            run_step(dependency.clone()).await?;
        }
    }

    // Wait for async dependencies
    for handle in handles {
        handle.await.unwrap();
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
            .spawn()?
            .wait()
            .await?;
    }
    Ok(())
}
