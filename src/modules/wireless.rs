// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::module::{Bar, RunPtr};
use crate::netlink::{self, WirelessState};
use crate::{Config as MainConfig, ModuleMsg};
use serde::{Deserialize, Serialize};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};
use tracing::{debug, instrument};

const PLACEHOLDER: &str = "-";
const TICK_RATE: Duration = Duration::from_millis(500);
const DISPLAY: Display = Display::Signal;
const MAX_ESSID_LEN: usize = 10;
const INTERFACE: &str = "wlan0";
const LABEL: &str = "wle";
const DISCONNECTED_LABEL: &str = ".wl";
const FORMAT: &str = "%l:%v";

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
enum Display {
    Essid,
    Signal,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    tick: Option<u32>,
    display: Option<Display>,
    max_essid_len: Option<usize>,
    interface: Option<String>,
    placeholder: Option<String>,
    label: Option<String>,
    disconnected_label: Option<String>,
    format: Option<String>,
}

#[derive(Debug)]
pub struct InternalConfig<'a> {
    display: Display,
    max_essid_len: usize,
    interface: &'a str,
    tick: Duration,
    label: &'a str,
    disconnected_label: &'a str,
}

impl<'a> From<&'a MainConfig> for InternalConfig<'a> {
    fn from(config: &'a MainConfig) -> Self {
        let mut tick = TICK_RATE;
        let mut display = DISPLAY;
        let mut max_essid_len = MAX_ESSID_LEN;
        let mut interface = INTERFACE;
        let mut label = LABEL;
        let mut disconnected_label = DISCONNECTED_LABEL;
        if let Some(c) = &config.wireless {
            if let Some(t) = c.tick {
                tick = Duration::from_millis(t as u64)
            }
            if let Some(d) = &c.display {
                display = *d
            }
            if let Some(m) = c.max_essid_len {
                max_essid_len = m
            }
            if let Some(i) = &c.interface {
                interface = i
            }
            if let Some(v) = &c.label {
                label = v;
            }
            if let Some(v) = &c.disconnected_label {
                disconnected_label = v;
            }
        };
        InternalConfig {
            display,
            max_essid_len,
            interface,
            tick,
            label,
            disconnected_label,
        }
    }
}

#[derive(Debug)]
pub struct Wireless<'a> {
    placeholder: &'a str,
    format: &'a str,
}

impl<'a> Wireless<'a> {
    pub fn with_config(config: &'a MainConfig) -> Self {
        let mut placeholder = PLACEHOLDER;
        let mut format = FORMAT;
        if let Some(c) = &config.wireless {
            if let Some(p) = &c.placeholder {
                placeholder = p
            }
            if let Some(v) = &c.format {
                format = v;
            }
        }
        Wireless {
            placeholder,
            format,
        }
    }
}

impl<'a> Bar for Wireless<'a> {
    fn name(&self) -> &str {
        "wireless"
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
pub fn run(key: char, main_config: MainConfig, tx: Sender<ModuleMsg>) -> Result<(), Error> {
    let config = InternalConfig::from(&main_config);
    debug!("{:#?}", config);
    let mut iteration_start: Instant;
    let mut iteration_end: Duration;
    loop {
        iteration_start = Instant::now();
        let label;
        let mut essid = "".to_owned();
        let mut signal = None;
        if let Some(state) = netlink::wireless_data(config.interface) {
            if let WirelessState::Connected(data) = state {
                label = config.label;
                if let Some(strength) = data.signal {
                    signal = Some(strength);
                };
                if let Some(val) = data.essid {
                    essid = if val.chars().count() > config.max_essid_len {
                        val[..config.max_essid_len].to_owned()
                    } else {
                        val
                    }
                }
            } else {
                label = config.disconnected_label;
            }
            match config.display {
                Display::Essid => tx.send(ModuleMsg(key, Some(essid), Some(label.to_string())))?,
                Display::Signal => {
                    if let Some(s) = signal {
                        tx.send(ModuleMsg(
                            key,
                            Some(format!("{:3}%", s)),
                            Some(label.to_string()),
                        ))?;
                    } else {
                        tx.send(ModuleMsg(
                            key,
                            Some("  ?%".to_string()),
                            Some(label.to_string()),
                        ))?;
                    }
                }
            }
        }
        iteration_end = iteration_start.elapsed();
        if iteration_end < config.tick {
            thread::sleep(config.tick - iteration_end);
        }
    }
}
