// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use anyhow::{Context, Result};
use baru::cli::Cli;
use baru::pulse::Pulse;
use baru::{trace, util, Baru, Config};
use clap::Parser;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tracing::{debug, error, info};

const XDG_CONFIG_HOME: &str = "XDG_CONFIG_HOME";
const APP_DIR: &str = "baru";
const CONFIG_FILE: &str = "baru.yaml";
const TICK_RATE: Duration = Duration::from_millis(50);

fn main() -> Result<()> {
    let cli = Cli::parse();
    trace::init(cli.logs).context("failed to init tracing")?;

    let home = env::var("HOME")?;
    let mut config_dir = env::var(XDG_CONFIG_HOME)
        .map(PathBuf::from)
        .unwrap_or_else(|_| Path::new(&home).join(".config"));
    config_dir.push(APP_DIR);
    util::check_dir(&config_dir)?;

    let config_file = config_dir.join(CONFIG_FILE);
    info!("config file: {:?}", config_file);
    let content = fs::read_to_string(config_file)
        .inspect_err(|e| error!("failed to read config file: {}", e))?;
    let config: Config = serde_yaml::from_str(&content)
        .inspect_err(|e| error!("failed to parse config file: {}", e))?;
    debug!("{:#?}", config);

    let tick = match config.tick {
        Some(ms) => Duration::from_millis(ms as u64),
        None => TICK_RATE,
    };
    let pulse = Arc::new(Mutex::new(Pulse::new(&config).inspect_err(|e| {
        error!("baru: error while creating pulse module, {}", e);
    })?));
    let mut baru = Baru::with_config(&config, &pulse)
        .inspect_err(|e| error!("failed to create baru instance {}", e))?;
    info!("baru instance initialized");

    let modules = baru.modules();
    info!("modules registered: {}", modules.len());
    debug!("modules: {:?}", modules);

    baru.start()
        .inspect_err(|e| error!("failed to start {}", e))?;
    info!("started");
    let mut iteration_start: Instant;
    let mut iteration_end: Duration;

    info!("launching main loop");
    loop {
        iteration_start = Instant::now();
        baru.update()
            .inspect_err(|e| error!("failed to update: {}", e))?;
        iteration_end = iteration_start.elapsed();
        if iteration_end < tick {
            thread::sleep(tick - iteration_end);
        }
    }
}
