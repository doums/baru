// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::module::{BaruMod, RunPtr};
use crate::nl_data::{self, WiredState};
use crate::Config as MainConfig;
use crate::Pulse;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const PLACEHOLDER: &str = "+@fn=1;󰈀+@fn=0;";
const TICK_RATE: Duration = Duration::from_millis(1000);
const INTERFACE: &str = "enp0s31f6";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    tick: Option<u32>,
    interface: Option<String>,
    discrete: Option<bool>,
    placeholder: Option<String>,
}

#[derive(Debug)]
pub struct InternalConfig<'a> {
    interface: &'a str,
    discrete: bool,
    tick: Duration,
}

impl<'a> From<&'a MainConfig> for InternalConfig<'a> {
    fn from(config: &'a MainConfig) -> Self {
        let mut tick = TICK_RATE;
        let mut interface = INTERFACE;
        let mut discrete = false;
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
        };
        InternalConfig {
            interface,
            discrete,
            tick,
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

impl<'a> BaruMod for Wired<'a> {
    fn run_fn(&self) -> RunPtr {
        run
    }

    fn placeholder(&self) -> &str {
        self.placeholder
    }
}

pub fn run(main_config: MainConfig, _: Arc<Mutex<Pulse>>, tx: Sender<String>) -> Result<(), Error> {
    let config = InternalConfig::from(&main_config);
    loop {
        let mut icon = "󰈂";
        if let WiredState::Connected = nl_data::wired_data(&config.interface) {
            icon = "󰈁";
        } else if config.discrete {
            tx.send("".to_string())?;
            return Ok(());
        }
        tx.send(format!(
            "{}{}{}",
            main_config.icon_font, icon, main_config.default_font
        ))?;
        thread::sleep(config.tick);
    }
}
