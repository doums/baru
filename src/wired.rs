// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::module::{Bar, RunPtr};
use crate::nl_data::{self, WiredState};
use crate::Pulse;
use crate::{Config as MainConfig, ModuleMsg};
use serde::{Deserialize, Serialize};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const PLACEHOLDER: &str = "-";
const TICK_RATE: Duration = Duration::from_millis(1000);
const INTERFACE: &str = "enp0s31f6";
const LABEL: &str = "wir";
const DISCONNECTED_LABEL: &str = ".wi";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    tick: Option<u32>,
    interface: Option<String>,
    discrete: Option<bool>,
    placeholder: Option<String>,
    text: Option<String>,
    disconnected_text: Option<String>,
}

#[derive(Debug)]
pub struct InternalConfig<'a> {
    interface: &'a str,
    discrete: bool,
    tick: Duration,
    text: &'a str,
    disconnected_text: &'a str,
}

impl<'a> From<&'a MainConfig> for InternalConfig<'a> {
    fn from(config: &'a MainConfig) -> Self {
        let mut tick = TICK_RATE;
        let mut interface = INTERFACE;
        let mut discrete = false;
        let mut text = LABEL;
        let mut disconnected_text = DISCONNECTED_LABEL;
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
            if let Some(v) = &c.text {
                text = v;
            }
            if let Some(v) = &c.disconnected_text {
                disconnected_text = v;
            }
        };
        InternalConfig {
            interface,
            discrete,
            tick,
            text,
            disconnected_text,
        }
    }
}

#[derive(Debug)]
pub struct Wired<'a> {
    placeholder: &'a str,
    config: &'a MainConfig,
}

impl<'a> Wired<'a> {
    pub fn with_config(config: &'a MainConfig) -> Self {
        let mut placeholder = PLACEHOLDER;
        if let Some(c) = &config.wired {
            if let Some(p) = &c.placeholder {
                placeholder = p
            }
        }
        Wired {
            placeholder,
            config,
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
}

pub fn run(
    key: char,
    main_config: MainConfig,
    _: Arc<Mutex<Pulse>>,
    tx: Sender<ModuleMsg>,
) -> Result<(), Error> {
    let config = InternalConfig::from(&main_config);
    loop {
        if let WiredState::Connected = nl_data::wired_data(&config.interface) {
            tx.send(ModuleMsg(key, config.text.to_string()))?;
        } else if config.discrete {
            tx.send(ModuleMsg(key, "".to_string()))?;
        } else {
            tx.send(ModuleMsg(key, config.disconnected_text.to_string()))?;
        }
        thread::sleep(config.tick);
    }
}
