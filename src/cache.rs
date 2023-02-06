use std::time::{SystemTime, UNIX_EPOCH};

use tokio::{fs::OpenOptions, io::AsyncWriteExt};

use crate::build::BuildError;

pub struct Cache {
    pub last_run: Option<u64>,
}

pub async fn load_cache(step: Option<String>) -> Result<Cache, BuildError> {
    let step = step.unwrap_or_else(|| "default".to_string());
    let last_run = match tokio::fs::read_to_string(format!(".doer/last_run/{step}")).await {
        Ok(last_run) => last_run.parse().ok(),
        Err(_) => {
            tokio::fs::create_dir_all(".doer/last_run").await?;
            tokio::fs::File::create(format!(".doer/last_run/{step}")).await?;
            None
        }
    };
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(format!(".doer/last_run/{step}"))
        .await?;
    let current = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    file.write_all(current.to_string().as_bytes()).await?;
    Ok(Cache { last_run })
}
