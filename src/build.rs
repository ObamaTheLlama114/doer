use std::{collections::HashMap, path::PathBuf};

use serde::Deserialize;

// Structs for deserializing build file
#[derive(Debug, Deserialize, Clone)]
struct SerdeBuild {
    default: Option<String>,
    step: Option<HashMap<String, SerdeStep>>,
}

#[derive(Debug, Deserialize, Clone)]
struct SerdeStep {
    command: Option<String>,
    #[serde(rename = "async")]
    asynch: Option<bool>,
    dependencies: Option<Vec<String>>,
    in_order: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct Step {
    pub command: Option<String>,
    pub dir: String,
    pub asynch: bool,
    pub dependencies: Vec<Step>,
    pub in_order: bool,
}

pub fn get_step(step_name: Option<String>, path: &str) -> Step {
    // Get full path to build file
    let path = get_full_path(path);

    let mut files: HashMap<String, SerdeBuild> = HashMap::new();

    // Read build file or exit if it does not exist
    let file = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        println!("Could not read file: {}", path);
        std::process::exit(1);
    });

    // Deserialize build file or exit if it fails
    let build: SerdeBuild = toml::from_str(&file).unwrap_or_else(|_| {
        println!("Could not parse file: {}", path);
        std::process::exit(1);
    });

    let step_name = step_name.unwrap_or_else(|| {
        build.default.clone().unwrap_or_else(|| {
            if build
                .step
                .as_ref()
                .unwrap_or(&HashMap::new())
                .contains_key("default")
            {
                "default".to_string()
            } else {
                println!("No default step found");
                std::process::exit(1)
            }
        })
    });
    files.insert(path.to_string(), build);
    get_step_inner(&step_name, &path, &mut files)
}

fn get_step_inner(step_name: &str, path: &str, files: &mut HashMap<String, SerdeBuild>) -> Step {
    // Get deserialized build file from cache or read from disk
    let build_file = if let Some(x) = files.get(path) {
        x.clone()
    } else {
        let file = std::fs::read_to_string(path).unwrap_or_else(|_| {
            println!("Could not read file: {}", path);
            std::process::exit(1);
        });
        let build: SerdeBuild = toml::from_str(&file).unwrap_or_else(|_| {
            println!("Could not parse file: {}", path);
            std::process::exit(1);
        });
        files.insert(path.to_string(), build.clone());
        build
    };

    let step_name = step_name.split(':').collect::<Vec<&str>>();

    // Check if step name is valid
    if step_name.is_empty() {
        println!("Step name is empty");
        std::process::exit(1);
    }
    if step_name.len() == 1 {
        // If step name is only one part, get step from current build file
        let step = build_file.step.unwrap_or_default();
        let step = step.get(step_name[0]).unwrap_or_else(|| {
            println!(
                "Step not found in file:\nstep: {}\nfile: {}",
                step_name[0], path
            );
            std::process::exit(1);
        });
        generate_step(step, path, files)
    } else {
        // If step name is multiple parts, get child build file and get step from that
        let path = get_child_path(path, step_name[0]);
        get_step_inner(&step_name[1..].join(":"), &path, files)
    }
}

fn generate_step(step: &SerdeStep, path: &str, files: &mut HashMap<String, SerdeBuild>) -> Step {
    // Generate a usable step from a deserialized step
    Step {
        command: step.command.clone(),
        dir: path.to_string(),
        asynch: step.asynch.unwrap_or(false),
        dependencies: generate_dependencies(step.dependencies.clone(), files, path),
        in_order: step.in_order.unwrap_or(false),
    }
}

fn generate_dependencies(
    dependencies: Option<Vec<String>>,
    files: &mut HashMap<String, SerdeBuild>,
    path: &str,
) -> Vec<Step> {
    if let Some(dependencies) = dependencies {
        // If dependencies exist, generate steps from them
        dependencies
            .iter()
            .filter(|x| !x.is_empty())
            .map(|x| {
                let x = x.split(':').collect::<Vec<&str>>();
                if x.len() == 1 {
                    get_step_inner(x[0], path, files)
                } else {
                    let path = get_child_path(path, x[0]);
                    get_step_inner(x[1], &path, files)
                }
            })
            .collect::<Vec<Step>>()
    } else {
        // If no dependencies, return empty vector
        Vec::new()
    }
}

fn get_full_path(path: &str) -> String {
    // Get full path to build file
    let mut path = PathBuf::from(path)
        .canonicalize()
        .unwrap_or_else(|_| {
            println!("Could not canonicalize path: {}", path);
            std::process::exit(1);
        })
        .to_str()
        .unwrap_or_else(|| {
            println!("Could not convert path to string: {}", path);
            std::process::exit(1);
        })
        .to_string();

    // Add build file name if not already there
    if !path.ends_with(".toml") {
        path.push_str("/build.toml");
    }
    path
}

fn get_child_path(path: &str, child: &str) -> String {
    // Get path to child build file
    if child == ".." {
        // If child is "..", get parent path instead
        return get_parent_path(path);
    }
    let path = PathBuf::from(path)
        .parent()
        .unwrap_or_else(|| {
            println!("Could not get parent of path: {}", path);
            std::process::exit(1);
        })
        .join(child)
        .canonicalize()
        .unwrap_or_else(|_| {
            println!("Could not canonicalize path: {}", path);
            std::process::exit(1);
        });

    if path.is_dir() {
        path.clone()
            .join("build.toml")
            .canonicalize()
            .unwrap_or_else(|_| {
                println!(
                    "Could not canonicalize path: {}",
                    path.to_str().unwrap_or("")
                );
                std::process::exit(1)
            })
            .to_str()
            .unwrap_or_else(|| {
                println!(
                    "Could not convert path to string: {}",
                    path.to_str().unwrap_or("")
                );
                std::process::exit(1)
            })
            .to_string()
    } else {
        path.to_str()
            .unwrap_or_else(|| {
                println!(
                    "Could not convert path to string: {}",
                    path.to_str().unwrap_or("")
                );
                std::process::exit(1)
            })
            .to_string()
    }
}

fn get_parent_path(path: &str) -> String {
    PathBuf::from(path)
        .parent()
        .unwrap_or_else(|| {
            println!("Could not get parent of path: {}", path);
            std::process::exit(1);
        })
        .to_str()
        .unwrap_or_else(|| {
            println!("Could not convert path to string: {}", path);
            std::process::exit(1);
        })
        .to_string()
}
