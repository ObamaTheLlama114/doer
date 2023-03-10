use std::{collections::HashMap, path::PathBuf};

use serde::Deserialize;

// Structs for deserializing build file
#[derive(Debug, Deserialize, Clone)]
struct SerdeBuild {
    default: Option<String>,
    step: Option<HashMap<String, SerdeStep>>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
enum SerdeDependency {
    String(String),
    Array(Vec<String>),
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
enum SerdeCommand {
    String(String),
    Array(Vec<String>),
}

#[derive(Debug, Deserialize, Clone)]
struct SerdeStep {
    command: Option<SerdeCommand>,
    env: Option<HashMap<String, String>>,
    #[serde(rename = "async")]
    asynch: Option<bool>,
    #[serde(rename = "depends")]
    dependencies: Option<SerdeDependency>,
    in_order: Option<bool>,
    quiet: Option<bool>,
    watch: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct Step {
    pub command: Vec<String>,
    pub env: HashMap<String, String>,
    pub dir: String,
    pub asynch: bool,
    pub dependencies: Vec<Step>,
    pub in_order: bool,
    pub quiet: bool,
    pub watch: Option<Vec<String>>,
}

#[derive(Debug)]
pub enum BuildError {
    IoError(std::io::Error),
    TomlError(toml::de::Error),
    JoinError(tokio::task::JoinError),
    ParseIntError(std::num::ParseIntError),
    SystemTimeError(std::time::SystemTimeError),
    MissingStep(String),
    InvalidPath(String),
    InvalidStep(String),
}

impl From<std::io::Error> for BuildError {
    fn from(error: std::io::Error) -> Self {
        BuildError::IoError(error)
    }
}

impl From<toml::de::Error> for BuildError {
    fn from(error: toml::de::Error) -> Self {
        BuildError::TomlError(error)
    }
}

impl From<tokio::task::JoinError> for BuildError {
    fn from(error: tokio::task::JoinError) -> Self {
        BuildError::JoinError(error)
    }
}

impl From<std::num::ParseIntError> for BuildError {
    fn from(error: std::num::ParseIntError) -> Self {
        BuildError::ParseIntError(error)
    }
}

impl From<std::time::SystemTimeError> for BuildError {
    fn from(error: std::time::SystemTimeError) -> Self {
        BuildError::SystemTimeError(error)
    }
}

type Result<T> = std::result::Result<T, BuildError>;

pub fn get_step(step_name: Option<String>, path: &str) -> Result<Step> {
    // Get full path to build file
    let path = get_full_path(path)?;
    let mut files: HashMap<String, SerdeBuild> = HashMap::new();
    get_step_inner(step_name, &path, &mut files)
}

fn get_step_inner(
    step_name: Option<String>,
    path: &str,
    files: &mut HashMap<String, SerdeBuild>,
) -> Result<Step> {
    // Get deserialized build file from cache or read from disk
    let build_file = load_file(path, files)?;

    let step_name = match step_name {
        Some(step_name) => step_name,
        None => match build_file.default {
            Some(step_name) => step_name,
            None => {
                if build_file
                    .step
                    .as_ref()
                    .unwrap_or(&HashMap::new())
                    .contains_key("default")
                {
                    "default".to_string()
                } else {
                    return Err(BuildError::MissingStep("default".to_string()));
                }
            }
        },
    };
    let step_name = step_name.split("::").collect::<Vec<&str>>();

    // Check if step name is valid
    if step_name.is_empty() {
        return Err(BuildError::InvalidStep(step_name.join("::")));
    }
    if step_name.len() == 1 {
        // If step name is only one part, get step from current build file
        let step = build_file.step.unwrap_or_default();
        let step = step.get(step_name[0]);
        if let Some(step) = step {
            generate_step(step, path, files)
        } else {
            let path = get_child_path(path, step_name[0]);
            if let Ok(path) = path {
                get_step_inner(None, &path, files)
            } else {
                Err(BuildError::MissingStep(step_name.join("::")))
            }
        }
    } else {
        // If step name is multiple parts, get child build file and get step from that
        let path = get_child_path(path, step_name[0])?;
        get_step_inner(Some(step_name[1..].join("::")), &path, files)
    }
}

fn generate_step(
    step: &SerdeStep,
    path: &str,
    files: &mut HashMap<String, SerdeBuild>,
) -> Result<Step> {
    // Generate a usable step from a deserialized step
    let command = match &step.command {
        Some(SerdeCommand::String(x)) => vec![x.to_string()],
        Some(SerdeCommand::Array(x)) => x.clone(),
        None => vec![],
    };

    let dependencies = match &step.dependencies {
        Some(SerdeDependency::String(x)) => Some(vec![x.to_string()]),
        Some(SerdeDependency::Array(x)) => Some(x.clone()),
        None => None,
    };

    Ok(Step {
        command,
        env: step.env.clone().unwrap_or_default(),
        dir: path.to_string(),
        asynch: step.asynch.unwrap_or(false),
        dependencies: generate_dependencies(dependencies, files, path)?,
        in_order: step.in_order.unwrap_or(false),
        quiet: step.quiet.unwrap_or(false),
        watch: step.watch.clone(),
    })
}

fn generate_dependencies(
    dependencies: Option<Vec<String>>,
    files: &mut HashMap<String, SerdeBuild>,
    path: &str,
) -> Result<Vec<Step>> {
    if let Some(dependencies) = dependencies {
        // If dependencies exist, generate steps from them
        let dependencies = dependencies
            .iter()
            .filter(|x| !x.is_empty())
            .map(|x| {
                let x = x.split("::").collect::<Vec<&str>>();
                if x.len() == 1 {
                    get_step_inner(Some(x[0].to_string()), path, files)
                } else {
                    let path = get_child_path(path, x[0])?;
                    get_step_inner(Some(x[1].to_string()), &path, files)
                }
            })
            .collect::<Result<Vec<Step>>>()?;
        Ok(dependencies)
    } else {
        // If no dependencies, return empty vector
        Ok(Vec::new())
    }
}

fn load_file(path: &str, files: &mut HashMap<String, SerdeBuild>) -> Result<SerdeBuild> {
    check_path(path)?;
    // Get deserialized build file from cache or read from disk
    if let Some(x) = files.get(path) {
        Ok(x.clone())
    } else {
        let file = std::fs::read_to_string(path)?;
        let build: SerdeBuild = toml::from_str(&file)?;
        files.insert(path.to_string(), build.clone());
        Ok(build)
    }
}

fn check_path(path: &str) -> Result<()> {
    // Check if path is valid
    let path_buf = PathBuf::from(path);
    if !path_buf.exists() {
        return Err(BuildError::InvalidPath(path.to_string()));
    }
    Ok(())
}

fn get_full_path(path: &str) -> Result<String> {
    check_path(path)?;
    // Get full path to build file
    let mut path = PathBuf::from(path)
        .canonicalize()?
        .to_str()
        .ok_or_else(|| BuildError::InvalidPath(path.to_owned()))?
        .to_string();

    // Add build file name if not already there
    if !path.ends_with(".toml") {
        path.push_str("/build.toml");
    }
    Ok(path)
}

fn get_child_path(path: &str, child: &str) -> Result<String> {
    // Get path to child build file
    let path_buf = PathBuf::from(path)
        .parent() // Remove build file from path
        .ok_or_else(|| BuildError::InvalidPath(path.to_owned()))?
        .join(child)
        .canonicalize()?;

    // Add build file name if not already there
    if path_buf.is_dir() {
        Ok(path_buf
            .join("build.toml")
            .canonicalize()?
            .to_str()
            .ok_or_else(|| BuildError::InvalidPath(path.to_owned()))?
            .to_string())
    } else {
        Ok(path_buf
            .to_str()
            .ok_or_else(|| BuildError::InvalidPath(path.to_owned()))?
            .to_string())
    }
}
