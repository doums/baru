// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::{read_and_parse, read_and_trim, BarModule, Config as MainConfig};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

const SYS_PATH: &str = "/sys/class/power_supply/";
const NAME: &str = "BAT0";
const LOW_LEVEL: u32 = 10;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    name: Option<String>,
    low_level: Option<u32>,
}

#[derive(Debug)]
pub struct Battery<'a> {
    name: &'a str,
    config: &'a MainConfig,
    low_level: u32,
}

impl<'a> Battery<'a> {
    pub fn with_config(config: &'a MainConfig) -> Self {
        let mut low_level = LOW_LEVEL;
        let mut name = NAME;
        if let Some(c) = &config.battery {
            if let Some(n) = &c.name {
                name = n;
            }
            if let Some(v) = &c.low_level {
                low_level = *v;
            }
        }
        Battery {
            name,
            config,
            low_level,
        }
    }
}

impl<'a> BarModule for Battery<'a> {
    fn refresh(&mut self) -> Result<String, Error> {
        let energy_full_design =
            read_and_parse(&format!("{}{}/energy_full_design", SYS_PATH, self.name))?;
        let energy_now = read_and_parse(&format!("{}{}/energy_now", SYS_PATH, self.name))?;
        let status = read_and_trim(&format!("{}{}/status", SYS_PATH, self.name))?;
        let capacity = energy_full_design as u64;
        let energy = energy_now as u64;
        let battery_level = u32::try_from(100_u64 * energy / capacity)?;
        let mut color = &self.config.default_color;
        if status != "Charging" && battery_level <= self.low_level {
            color = &self.config.red;
        }
        if status == "Full" {
            color = &self.config.green
        }
        Ok(format!(
            "{:3}% {}{}{}{}{}",
            battery_level,
            color,
            self.config.icon_font,
            get_battery_icon(&status, battery_level),
            self.config.default_font,
            self.config.default_color
        ))
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
