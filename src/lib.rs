// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use chrono::prelude::*;
mod error;
use error::Error;
use std::convert::TryFrom;
use std::fs;
use std::io;

const ENERGY_NOW: &'static str = "/sys/class/power_supply/BAT0/energy_now";
const POWER_STATUS: &'static str = "/sys/class/power_supply/BAT0/status";
const ENERGY_FULL_DESIGN: &'static str = "/sys/class/power_supply/BAT0/energy_full_design";
const CORETEMP_PATH: &'static str = "/sys/devices/platform/coretemp.0/hwmon/hwmon7";
const DEFAULT_FONT: &'static str = "+@fn=0;";
const ICON_FONT: &'static str = "+@fn=1;";
const DEFAULT_COLOR: &'static str = "+@fg=0;";
const RED: &'static str = "+@fg=1;";
const GREEN: &'static str = "+@fg=2;";

pub struct Bar<'a> {
    default_font: &'a str,
    icon: &'a str,
    default_color: &'a str,
    red: &'a str,
    green: &'a str,
}

impl<'a> Bar<'a> {
    pub fn new() -> Self {
        Bar {
            default_font: DEFAULT_FONT,
            icon: ICON_FONT,
            default_color: DEFAULT_COLOR,
            red: RED,
            green: GREEN,
        }
    }

    fn battery(self: &Self) -> Result<String, Error> {
        let energy_full_design = read_and_trim(ENERGY_FULL_DESIGN)?;
        let energy_now = read_and_trim(ENERGY_NOW)?;
        let status = read_and_trim(POWER_STATUS)?;
        let capacity = energy_full_design.parse::<u64>()?;
        let energy = energy_now.parse::<u64>()?;
        let battery_level = u32::try_from(100u64 * energy / capacity)?;
        let mut color = match battery_level {
            0..=10 => self.red,
            _ => self.default_color,
        };
        if status == "Full" {
            color = self.green
        }
        Ok(format!(
            "{}{}{}{}{} {}%",
            color,
            self.icon,
            get_battery_icon(&status.trim(), battery_level),
            self.default_font,
            self.default_color,
            battery_level
        ))
    }

    fn core_temperature(self: &Self) -> Result<String, Error> {
        let core_1_str = read_and_trim(&format!("{}/temp2_input", CORETEMP_PATH))?;
        let core_2_str = read_and_trim(&format!("{}/temp3_input", CORETEMP_PATH))?;
        let core_3_str = read_and_trim(&format!("{}/temp4_input", CORETEMP_PATH))?;
        let core_4_str = read_and_trim(&format!("{}/temp5_input", CORETEMP_PATH))?;
        let core_1 = core_1_str.parse::<f32>()?;
        let core_2 = core_2_str.parse::<f32>()?;
        let core_3 = core_3_str.parse::<f32>()?;
        let core_4 = core_4_str.parse::<f32>()?;
        let average = (((core_1 + core_2 + core_3 + core_4) / 4f32) / 1000f32).round() as i32;
        let mut color = self.default_color;
        let icon = match average {
            0..=50 => "󱃃",
            51..=70 => "󰔏",
            71..=100 => "󱃂",
            _ => "󰸁",
        };
        if average > 75 {
            color = self.red;
        }
        Ok(format!(
            "{}{}{}{}{} {}°",
            color, self.icon, icon, self.default_font, self.default_color, average
        ))
    }

    pub fn update(self: &Self) -> Result<(), Error> {
        let date_time = date_time();
        let battery = self.battery()?;
        let temperature = self.core_temperature()?;
        println!("{}  {}   {}", temperature, battery, date_time);
        Ok(())
    }
}

fn read_and_trim<'a>(file: &'a str) -> Result<String, io::Error> {
    let content = fs::read_to_string(file)?;
    Ok(content.trim().to_string())
}

fn date_time() -> String {
    let now = Local::now();
    now.format("%a. %-e %B %Y, %-kh%M").to_string()
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
