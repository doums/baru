// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::module::{Bar, RunPtr};
use crate::Pulse;
use crate::{read_and_parse, Config as MainConfig, ModuleMsg};
use serde::{Deserialize, Serialize};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const PLACEHOLDER: &str = "-";
const SYS_PATH: &str = "/sys/devices/pci0000:00/0000:00:02.0/drm/card0/card0-eDP-1/intel_backlight";
const TICK_RATE: Duration = Duration::from_millis(50);
const LABEL: &str = "bri";
const FORMAT: &str = "%l:%v";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    placeholder: Option<String>,
    sys_path: Option<String>,
    tick: Option<u32>,
    label: Option<String>,
    format: Option<String>,
}

#[derive(Debug, Clone)]
pub struct InternalConfig<'a> {
    sys_path: &'a str,
    tick: Duration,
    label: &'a str,
}

impl<'a> From<&'a MainConfig> for InternalConfig<'a> {
    fn from(config: &'a MainConfig) -> Self {
        let mut sys_path = SYS_PATH;
        let mut tick = TICK_RATE;
        let mut label = LABEL;
        if let Some(c) = &config.brightness {
            if let Some(v) = &c.sys_path {
                sys_path = &v;
            }
            if let Some(t) = c.tick {
                tick = Duration::from_millis(t as u64)
            }
            if let Some(v) = &c.label {
                label = v;
            }
        }
        InternalConfig {
            sys_path,
            tick,
            label,
        }
    }
}

#[derive(Debug)]
pub struct Brightness<'a> {
    placeholder: &'a str,
    config: &'a MainConfig,
    format: &'a str,
}

impl<'a> Brightness<'a> {
    pub fn with_config(config: &'a MainConfig) -> Self {
        let mut placeholder = PLACEHOLDER;
        let mut format = FORMAT;
        if let Some(c) = &config.brightness {
            if let Some(p) = &c.placeholder {
                placeholder = p
            }
            if let Some(v) = &c.format {
                format = v;
            }
        }
        Brightness {
            placeholder,
            config,
            format,
        }
    }
}

impl<'a> Bar for Brightness<'a> {
    fn name(&self) -> &str {
        "brightness"
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

pub fn run(
    key: char,
    main_config: MainConfig,
    _: Arc<Mutex<Pulse>>,
    tx: Sender<ModuleMsg>,
) -> Result<(), Error> {
    let config = InternalConfig::from(&main_config);
    loop {
        let brightness = read_and_parse(&format!("{}/actual_brightness", config.sys_path))?;
        let max_brightness = read_and_parse(&format!("{}/max_brightness", config.sys_path))?;
        let percentage = 100 * brightness / max_brightness;
        tx.send(ModuleMsg(
            key,
            Some(format!("{:3}%", percentage)),
            Some(config.label.to_string()),
        ))?;
        thread::sleep(config.tick);
    }
}
