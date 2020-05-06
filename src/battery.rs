// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::{read_and_parse, read_and_trim, BarModule, Config as MainConfig};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

const ENERGY_NOW: &'static str = "/sys/class/power_supply/BAT0/energy_now";
const STATUS: &'static str = "/sys/class/power_supply/BAT0/status";
const ENERGY_FULL_DESIGN: &'static str = "/sys/class/power_supply/BAT0/energy_full_design";
const LOW_LEVEL: u32 = 10;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    energy_full_design: Option<String>,
    energy_now: Option<String>,
    status: Option<String>,
    low_level: Option<u32>,
}

#[derive(Debug)]
pub struct Battery<'a> {
    energy_full_design: &'a str,
    energy_now: &'a str,
    status: &'a str,
    config: &'a MainConfig,
    low_level: u32,
}

impl<'a> Battery<'a> {
    pub fn with_config(config: &'a MainConfig) -> Self {
        let mut energy_full_design = ENERGY_FULL_DESIGN;
        let mut energy_now = ENERGY_NOW;
        let mut status = STATUS;
        let mut low_level = LOW_LEVEL;
        if let Some(c) = &config.battery {
            if let Some(v) = &c.energy_full_design {
                energy_full_design = &v;
            }
            if let Some(v) = &c.energy_now {
                energy_now = &v;
            }
            if let Some(v) = &c.status {
                status = &v;
            }
            if let Some(v) = &c.low_level {
                low_level = *v;
            }
        }
        Battery {
            energy_full_design,
            energy_now,
            status,
            config,
            low_level,
        }
    }
}

impl<'a> BarModule for Battery<'a> {
    fn refresh(&mut self) -> Result<String, Error> {
        let energy_full_design = read_and_parse(self.energy_full_design)?;
        let energy_now = read_and_parse(self.energy_now)?;
        let status = read_and_trim(self.status)?;
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
