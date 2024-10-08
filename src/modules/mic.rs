// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::module::{Bar, RunPtr};
use crate::pulse::PULSE;
use crate::{Config as MainConfig, ModuleMsg};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};
use tracing::{debug, error, instrument};

const PLACEHOLDER: &str = "-";
const TICK_RATE: Duration = Duration::from_millis(50);
const MUTE_LABEL: &str = ".mi";
const LABEL: &str = "mic";
const FORMAT: &str = "%l:%v";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub source_name: Option<String>,
    tick: Option<u32>,
    placeholder: Option<String>,
    label: Option<String>,
    mute_label: Option<String>,
    format: Option<String>,
}

#[derive(Debug)]
pub struct InternalConfig<'a> {
    tick: Duration,
    label: &'a str,
    mute_label: &'a str,
}

impl<'a> From<&'a MainConfig> for InternalConfig<'a> {
    fn from(config: &'a MainConfig) -> Self {
        let mut tick = TICK_RATE;
        let mut label = LABEL;
        let mut mute_label = MUTE_LABEL;
        if let Some(c) = &config.mic {
            if let Some(t) = c.tick {
                tick = Duration::from_millis(t as u64)
            }
            if let Some(v) = &c.label {
                label = v;
            }
            if let Some(v) = &c.mute_label {
                mute_label = v;
            }
        }
        InternalConfig {
            tick,
            label,
            mute_label,
        }
    }
}

#[derive(Debug)]
pub struct Mic<'a> {
    placeholder: &'a str,
    format: &'a str,
}

impl<'a> Mic<'a> {
    pub fn with_config(config: &'a MainConfig) -> Self {
        let mut placeholder = PLACEHOLDER;
        let mut format = FORMAT;
        if let Some(c) = &config.mic {
            if let Some(p) = &c.placeholder {
                placeholder = p
            }
            if let Some(v) = &c.format {
                format = v;
            }
        }
        Mic {
            placeholder,
            format,
        }
    }
}

impl<'a> Bar for Mic<'a> {
    fn name(&self) -> &str {
        "mic"
    }

    fn run_fn(&self) -> RunPtr {
        run
    }

    fn placeholder(&self) -> &str {
        self.placeholder
    }

    fn format(&self) -> &str {
        self.format
    }
}

#[instrument(skip_all)]
pub fn run(
    running: &AtomicBool,
    key: char,
    main_config: MainConfig,
    tx: Sender<ModuleMsg>,
) -> Result<(), Error> {
    let config = InternalConfig::from(&main_config);
    debug!("{:#?}", config);
    let mut iteration_start: Instant;
    let mut iteration_end: Duration;
    let pulse = PULSE.get().ok_or("pulse module not initialized")?;
    while running.load(Ordering::Relaxed) {
        iteration_start = Instant::now();
        if let Some(data) = pulse
            .lock()
            .map_err(|e| {
                error!("failed to lock pulse module: {}", e);
                Error::new("failed to lock pulse module")
            })?
            .source_data()
        {
            let label = match data.1 {
                true => config.mute_label,
                false => config.label,
            };
            tx.send(ModuleMsg(
                key,
                Some(format!("{:3}%", data.0)),
                Some(label.to_string()),
            ))?;
        }
        iteration_end = iteration_start.elapsed();
        if iteration_end < config.tick {
            thread::sleep(config.tick - iteration_end);
        }
    }
    Ok(())
}
