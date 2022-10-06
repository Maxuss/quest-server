use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::bail;
use tokio::fs::{create_dir_all, remove_file, File};
use tracing::info;

macro_rules! ensure_create_dir {
    ($parent:ident/$this:ident) => {
        let $this = $parent.join(stringify!($this));
        if !$this.exists() {
            create_dir_all($this).await?;
        }
    };
}

#[tracing::instrument]
pub async fn prepare_fs() -> anyhow::Result<()> {
    info!("Preparing file system...");
    let path = PathBuf::from_str("data").unwrap(); // casual unwrapping is safe here because FromStr impl returns Infallible
    if !path.exists() {
        create_dir_all(&path).await?;
    }

    ensure_create_dir!(path / image);
    ensure_create_dir!(path / cache);

    Ok(())
}

#[tracing::instrument]
pub async fn create<P: AsRef<Path> + std::fmt::Debug>(path: P) -> anyhow::Result<File> {
    let path = path.as_ref();
    if !path.exists() {
        return File::create(path).await.map_err(anyhow::Error::from);
    }
    remove_file(path).await?;
    return File::create(path).await.map_err(anyhow::Error::from);
}

#[tracing::instrument]
pub async fn open<P: AsRef<Path> + std::fmt::Debug>(path: P) -> anyhow::Result<File> {
    let path = path.as_ref();
    if !path.exists() {
        return File::create(path).await.map_err(anyhow::Error::from);
    }
    bail!("File does not exist!")
}
