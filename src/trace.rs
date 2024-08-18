// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::path::{Path, PathBuf};
use std::{env, fs};

use anyhow::Result;
use tracing_appender::{non_blocking::WorkerGuard, rolling};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::EnvFilter;

use crate::cli::Logs;
use crate::util;

const XDG_CACHE_HOME: &str = "XDG_CACHE_HOME";
const LOG_DIR: &str = "baru";
const LOG_FILE: &str = "baru.log";
const LOG_FILE_OLD: &str = "baru.old.log";

fn rotate_log_file(log_dir: PathBuf) -> Result<()> {
    let log_file = log_dir.join(LOG_FILE);
    if log_file.is_file() {
        let old_file = log_dir.join(LOG_FILE_OLD);
        let data = fs::read(&log_file).inspect_err(|e| {
            eprintln!(
                "failed to read log file during log rotation {}: {e}",
                log_file.display()
            )
        })?;
        fs::write(&old_file, data).inspect_err(|e| {
            eprintln!(
                "failed to write log file during log rotation {}: {e}",
                old_file.display()
            )
        })?;
        fs::remove_file(log_file)?;
    }
    Ok(())
}

pub fn init(logs: Option<Logs>) -> Result<Option<WorkerGuard>> {
    let Some(l) = logs else {
        return Ok(None);
    };

    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env()
        .unwrap();

    match l {
        Logs::Stdout => {
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .compact()
                .init();
            Ok(None)
        }
        Logs::File => {
            let home = env::var("HOME")?;
            let cache_dir = env::var(XDG_CACHE_HOME)
                .map(PathBuf::from)
                .unwrap_or_else(|_| Path::new(&home).join(".cache"));
            let log_dir = cache_dir.join(LOG_DIR);
            util::check_dir(&log_dir)?;
            rotate_log_file(log_dir.clone()).ok();

            let appender = rolling::never(log_dir, LOG_FILE);
            let (writer, guard) = tracing_appender::non_blocking(appender);

            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .compact()
                .with_ansi(false)
                .with_writer(writer)
                .init();
            Ok(Some(guard))
        }
        _ => Ok(None),
    }
}
