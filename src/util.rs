// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use anyhow::{Context, Result};
use std::{fs, path::PathBuf};
use tracing::{debug, error};

use crate::error::Error;

/// Check if a directory exists, if not create it including all
/// parent components
pub fn check_dir(path: &PathBuf) -> Result<()> {
    if !path.is_dir() {
        debug!("directory `{}` does not exist, creating it", path.display());
        return fs::create_dir_all(path)
            .inspect_err(|e| error!("Failed to create directory `{}`: {e}", path.display()))
            .context(format!("Failed to create directory `{}`", path.display()));
    }
    Ok(())
}

pub fn read_and_trim(file: &str) -> Result<String, Error> {
    let content = fs::read_to_string(file)
        .inspect_err(|e| error!("failed to read the file `{}`: {}", file, e))?;
    Ok(content.trim().to_string())
}

pub fn read_and_parse(file: &str) -> Result<i32, Error> {
    let content = read_and_trim(file)?;
    let data = content
        .parse::<i32>()
        .inspect_err(|e| error!("failed to parse the file `{}`: {}", file, e))?;
    Ok(data)
}
