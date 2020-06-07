// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::module::BaruMod;
use crate::Pulse;
use crate::{read_and_parse, Config as MainConfig};
use serde::{Deserialize, Serialize};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const PLACEHOLDER: &str = "+@fn=1;󰃞+@fn=0;";
const SYS_PATH: &'static str =
    "/sys/devices/pci0000:00/0000:00:02.0/drm/card0/card0-eDP-1/intel_backlight";
const TICK_RATE: Duration = Duration::from_millis(50);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    placeholder: Option<String>,
    sys_path: Option<String>,
    tick: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct InternalConfig<'a> {
    sys_path: &'a str,
    tick: Duration,
}

impl<'a> From<&'a MainConfig> for InternalConfig<'a> {
    fn from(config: &'a MainConfig) -> Self {
        let mut sys_path = SYS_PATH;
        let mut tick = TICK_RATE;
        if let Some(c) = &config.brightness {
            if let Some(v) = &c.sys_path {
                sys_path = &v;
            }
            if let Some(t) = c.tick {
                tick = Duration::from_millis(t as u64)
            }
        }
        InternalConfig { sys_path, tick }
    }
}

#[derive(Debug)]
pub struct Brightness<'a> {
    placeholder: &'a str,
    config: &'a MainConfig,
}

impl<'a> Brightness<'a> {
    pub fn with_config(config: &'a MainConfig) -> Self {
        let mut placeholder = PLACEHOLDER;
        if let Some(c) = &config.brightness {
            if let Some(p) = &c.placeholder {
                placeholder = p
            }
        }
        Brightness {
            placeholder,
            config,
        }
    }
}

impl<'a> BaruMod for Brightness<'a> {
    fn run_fn(&self) -> fn(MainConfig, Arc<Mutex<Pulse>>, Sender<String>) -> Result<(), Error> {
        run
    }

    fn placeholder(&self) -> &str {
        self.placeholder
    }
}

pub fn run(main_config: MainConfig, _: Arc<Mutex<Pulse>>, tx: Sender<String>) -> Result<(), Error> {
    let config = InternalConfig::from(&main_config);
    loop {
        let brightness = read_and_parse(&format!("{}/actual_brightness", config.sys_path))?;
        let max_brightness = read_and_parse(&format!("{}/max_brightness", config.sys_path))?;
        let percentage = 100 * brightness / max_brightness;
        tx.send(format!(
            "{:3}%{}󰃟{}",
            percentage, main_config.icon_font, main_config.default_font
        ))?;
        thread::sleep(config.tick);
    }
}
