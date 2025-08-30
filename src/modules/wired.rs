// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::module::{Bar, RunPtr};
use crate::netlink::{self, WiredState};
use crate::{Config as MainConfig, ModuleMsg};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};
use tracing::{debug, instrument, warn};

const PLACEHOLDER: &str = "-";
const TICK_RATE: Duration = Duration::from_millis(1000);
const INTERFACE: &str = "enp0s31f6";
const DISCRETE: bool = false;
const LABEL: &str = "eth";
const DISCONNECTED_LABEL: &str = ".et";
const FORMAT: &str = "%l";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    tick: Option<u32>,
    interface: Option<String>,
    discrete: Option<bool>,
    placeholder: Option<String>,
    label: Option<String>,
    disconnected_label: Option<String>,
    format: Option<String>,
}

#[derive(Debug)]
pub struct InternalConfig<'a> {
    interface: &'a str,
    discrete: bool,
    tick: Duration,
    label: &'a str,
    disconnected_label: &'a str,
}

impl<'a> From<&'a MainConfig> for InternalConfig<'a> {
    fn from(config: &'a MainConfig) -> Self {
        let mut tick = TICK_RATE;
        let mut interface = INTERFACE;
        let mut discrete = DISCRETE;
        let mut label = LABEL;
        let mut disconnected_label = DISCONNECTED_LABEL;
        if let Some(c) = &config.wired {
            if let Some(t) = c.tick {
                tick = Duration::from_millis(t as u64)
            }
            if let Some(i) = &c.interface {
                interface = i
            }
            if let Some(b) = c.discrete {
                discrete = b;
            }
            if let Some(v) = &c.label {
                label = v;
            }
            if let Some(v) = &c.disconnected_label {
                disconnected_label = v;
            }
        };
        InternalConfig {
            interface,
            discrete,
            tick,
            label,
            disconnected_label,
        }
    }
}

#[derive(Debug)]
pub struct Wired<'a> {
    placeholder: &'a str,
    format: &'a str,
}

impl<'a> Wired<'a> {
    pub fn with_config(config: &'a MainConfig) -> Self {
        let mut placeholder = PLACEHOLDER;
        let mut format = FORMAT;
        if let Some(c) = &config.wired {
            if let Some(p) = &c.placeholder {
                placeholder = p
            }
            if let Some(v) = &c.format {
                format = v;
            }
        }
        Wired {
            placeholder,
            format,
        }
    }
}

impl<'a> Bar for Wired<'a> {
    fn name(&self) -> &str {
        "wired"
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
    let mut no_data_logged = false;
    while running.load(Ordering::Relaxed) {
        iteration_start = Instant::now();
        let data = netlink::wired_data(config.interface);
        if !no_data_logged && data.is_none() {
            warn!("no data for interface: {}", config.interface);
            no_data_logged = true;
        }
        match data {
            Some(WiredState::Connected) => {
                tx.send(ModuleMsg(key, None, Some(config.label.to_string())))?;
            }
            _ => {
                if config.discrete {
                    tx.send(ModuleMsg(key, None, None))?;
                } else {
                    tx.send(ModuleMsg(
                        key,
                        None,
                        Some(config.disconnected_label.to_string()),
                    ))?;
                }
            }
        }
        iteration_end = iteration_start.elapsed();
        if iteration_end < config.tick {
            thread::sleep(config.tick - iteration_end);
        }
    }
    Ok(())
}
