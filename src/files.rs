use anyhow::Result;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

pub fn dir_to_hashmap(dir: &Path) -> Result<HashMap<String, PathBuf>> {
    Ok(fs::read_dir(dir)?
        .filter_map(|x| {
            Some((
                x.as_ref()
                    .ok()?
                    .path()
                    .file_stem()?
                    .to_string_lossy()
                    .into_owned(),
                x.ok()?.path(),
            ))
        })
        .collect())
}
