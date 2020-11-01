// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::module::{BaruMod, RunPtr};
use crate::pulse::Pulse;
use crate::Config as MainConfig;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const PLACEHOLDER: &str = "+@fn=1;󰸈+@fn=0;";
const HIGH_LEVEL: u32 = 100;
const TICK_RATE: Duration = Duration::from_millis(50);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub index: Option<u32>,
    high_level: Option<u32>,
    tick: Option<u32>,
    placeholder: Option<String>,
}

#[derive(Debug)]
pub struct InternalConfig {
    high_level: u32,
    tick: Duration,
}

impl<'a> From<&'a MainConfig> for InternalConfig {
    fn from(config: &'a MainConfig) -> Self {
        let mut tick = TICK_RATE;
        let mut high_level = HIGH_LEVEL;
        if let Some(c) = &config.sound {
            if let Some(v) = c.high_level {
                high_level = v;
            }
            if let Some(t) = c.tick {
                tick = Duration::from_millis(t as u64)
            }
        }
        InternalConfig { high_level, tick }
    }
}

#[derive(Debug)]
pub struct Sound<'a> {
    placeholder: &'a str,
    config: &'a MainConfig,
}

impl<'a> Sound<'a> {
    pub fn with_config(config: &'a MainConfig) -> Self {
        let mut placeholder = PLACEHOLDER;
        if let Some(c) = &config.sound {
            if let Some(p) = &c.placeholder {
                placeholder = p
            }
        }
        Sound {
            placeholder,
            config,
        }
    }
}

impl<'a> BaruMod for Sound<'a> {
    fn run_fn(&self) -> RunPtr {
        run
    }

    fn placeholder(&self) -> &str {
        self.placeholder
    }
}

pub fn run(
    main_config: MainConfig,
    pulse: Arc<Mutex<Pulse>>,
    tx: Sender<String>,
) -> Result<(), Error> {
    let config = InternalConfig::from(&main_config);
    loop {
        if let Some(data) = pulse.lock().unwrap().output_data() {
            let mut color = &main_config.default_color;
            let icon = if data.1 {
                "󰸈"
            } else {
                match data.0 {
                    0..=9 => "󰕿",
                    10..=40 => "󰖀",
                    _ => "󰕾",
                }
            };
            if data.0 > config.high_level as i32 {
                color = &main_config.red;
            }
            tx.send(format!(
                "{:3}%{}{}{}{}{}",
                data.0,
                color,
                main_config.icon_font,
                icon,
                main_config.default_font,
                main_config.default_color
            ))?;
        }
        thread::sleep(config.tick);
    }
}
