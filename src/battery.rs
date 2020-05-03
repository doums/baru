use crate::error::Error;
use crate::{read_and_parse, read_and_trim, Config, Refresh};
use std::convert::TryFrom;

const ENERGY_NOW: &'static str = "/sys/class/power_supply/BAT0/energy_now";
const POWER_STATUS: &'static str = "/sys/class/power_supply/BAT0/status";
const ENERGY_FULL_DESIGN: &'static str = "/sys/class/power_supply/BAT0/energy_full_design";

#[derive(Debug)]
pub struct Battery<'a> {
    energy_full_design: &'a str,
    energy_now: &'a str,
    status: &'a str,
    config: &'a Config,
}

impl<'a> Battery<'a> {
    pub fn with_config(config: &'a Config) -> Self {
        Battery {
            energy_full_design: match &config.energy_full_design {
                Some(val) => &val,
                None => ENERGY_FULL_DESIGN,
            },
            energy_now: match &config.energy_now {
                Some(val) => &val,
                None => ENERGY_NOW,
            },
            status: match &config.power_status {
                Some(val) => &val,
                None => POWER_STATUS,
            },
            config,
        }
    }
}

impl<'a> Refresh for Battery<'a> {
    fn refresh(&mut self) -> Result<String, Error> {
        let energy_full_design = read_and_parse(ENERGY_FULL_DESIGN)?;
        let energy_now = read_and_parse(ENERGY_NOW)?;
        let status = read_and_trim(POWER_STATUS)?;
        let capacity = energy_full_design as u64;
        let energy = energy_now as u64;
        let battery_level = u32::try_from(100_u64 * energy / capacity)?;
        let mut color = match battery_level {
            0..=10 => {
                if status == "Discharging" {
                    &self.config.red
                } else {
                    &self.config.default_color
                }
            }
            _ => &self.config.default_color,
        };
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
