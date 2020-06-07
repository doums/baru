// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::module::BaruMod;
use crate::pulse::Pulse;
use crate::{read_and_parse, read_and_trim, Config as MainConfig};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const PLACEHOLDER: &str = "+@fn=1;󱃍+@fn=0;";
const SYS_PATH: &str = "/sys/class/power_supply/";
const NAME: &str = "BAT0";
const LOW_LEVEL: u32 = 10;
const TICK_RATE: Duration = Duration::from_millis(500);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    name: Option<String>,
    low_level: Option<u32>,
    full_design: Option<bool>,
    tick: Option<u32>,
    placeholder: Option<String>,
}

#[derive(Debug)]
pub struct InternalConfig<'a> {
    name: &'a str,
    low_level: u32,
    full_design: bool,
    tick: Duration,
}

impl<'a> From<&'a MainConfig> for InternalConfig<'a> {
    fn from(config: &'a MainConfig) -> Self {
        let mut low_level = LOW_LEVEL;
        let mut name = NAME;
        let mut full_design = false;
        let mut tick = TICK_RATE;
        if let Some(c) = &config.battery {
            if let Some(n) = &c.name {
                name = n;
            }
            if let Some(v) = &c.low_level {
                low_level = *v;
            }
            if let Some(b) = c.full_design {
                if b {
                    full_design = true;
                }
            }
            if let Some(t) = c.tick {
                tick = Duration::from_millis(t as u64)
            }
        }
        InternalConfig {
            name,
            low_level,
            full_design,
            tick,
        }
    }
}

#[derive(Debug)]
pub struct Battery<'a> {
    placeholder: &'a str,
    config: &'a MainConfig,
}

impl<'a> Battery<'a> {
    pub fn with_config(config: &'a MainConfig) -> Self {
        let mut placeholder = PLACEHOLDER;
        if let Some(c) = &config.battery {
            if let Some(p) = &c.placeholder {
                placeholder = p
            }
        }
        Battery {
            placeholder,
            config,
        }
    }
}

impl<'a> BaruMod for Battery<'a> {
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
        let energy_full = match config.full_design {
            true => read_and_parse(&format!("{}{}/energy_full_design", SYS_PATH, config.name))?,
            false => read_and_parse(&format!("{}{}/energy_full", SYS_PATH, config.name))?,
        };
        let energy_now = read_and_parse(&format!("{}{}/energy_now", SYS_PATH, config.name))?;
        let status = read_and_trim(&format!("{}{}/status", SYS_PATH, config.name))?;
        let capacity = energy_full as u64;
        let energy = energy_now as u64;
        let battery_level = u32::try_from(100_u64 * energy / capacity)?;
        let mut color = &main_config.default_color;
        if status != "Charging" && battery_level <= config.low_level {
            color = &main_config.red;
        }
        if status == "Full" {
            color = &main_config.green
        }
        tx.send(format!(
            "{:3}%{}{}{}{}{}",
            battery_level,
            color,
            main_config.icon_font,
            get_battery_icon(&status, battery_level),
            main_config.default_font,
            main_config.default_color
        ))?;
        thread::sleep(config.tick);
    }
}

fn get_battery_icon<'a>(state: &'a str, level: u32) -> &'static str {
    match state {
        "Full" => "󰁹",
        "Discharging" => match level {
            0..=9 => "󰂎",
            10..=19 => "󰁺",
            20..=29 => "󰁻",
            30..=39 => "󰁼",
            40..=49 => "󰁽",
            50..=59 => "󰁾",
            60..=69 => "󰁿",
            70..=79 => "󰂀",
            80..=89 => "󰂁",
            90..=99 => "󰂂",
            100 => "󰁹",
            _ => "󱃍",
        },
        "Charging" => match level {
            0..=9 => "󰢟",
            10..=19 => "󰢜",
            20..=29 => "󰂆",
            30..=39 => "󰂇",
            40..=49 => "󰂈",
            50..=59 => "󰢝",
            60..=69 => "󰂉",
            70..=79 => "󰢞",
            80..=89 => "󰂊",
            90..=99 => "󰂋",
            100 => "󰂅",
            _ => "󱃍",
        },
        _ => "󱃍",
    }
}
