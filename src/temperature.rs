// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::{read_and_parse, BarModule, Config};
use std::fs;

const CORETEMP_PATH: &'static str = "/sys/devices/platform/coretemp.0/hwmon";

#[derive(Debug)]
pub struct Temperature<'a> {
    coretemp_path: String,
    config: &'a Config,
}

impl<'a> Temperature<'a> {
    pub fn with_config(config: &'a Config) -> Result<Self, Error> {
        let path = match &config.backlight {
            Some(val) => &val,
            None => CORETEMP_PATH,
        };
        Ok(Temperature {
            coretemp_path: find_temp_dir(path)?,
            config,
        })
    }
}

impl<'a> BarModule for Temperature<'a> {
    fn refresh(&mut self) -> Result<String, Error> {
        let core_1 = read_and_parse(&format!("{}/temp2_input", self.coretemp_path))?;
        let core_2 = read_and_parse(&format!("{}/temp3_input", self.coretemp_path))?;
        let core_3 = read_and_parse(&format!("{}/temp4_input", self.coretemp_path))?;
        let core_4 = read_and_parse(&format!("{}/temp5_input", self.coretemp_path))?;
        let average =
            (((core_1 + core_2 + core_3 + core_4) as f32 / 4_f32) / 1000_f32).round() as i32;
        let mut color = &self.config.default_color;
        let icon = match average {
            0..=49 => "󱃃",
            50..=69 => "󰔏",
            70..=100 => "󱃂",
            _ => "󰸁",
        };
        if average > 75 {
            color = &self.config.red;
        }
        Ok(format!(
            "{:3}° {}{}{}{}{}",
            average,
            color,
            self.config.icon_font,
            icon,
            self.config.default_font,
            self.config.default_color
        ))
    }
}

fn find_temp_dir<'a>(str_path: &'a str) -> Result<String, Error> {
    let entries = fs::read_dir(str_path).map_err(|err| {
        format!(
            "error while reading the directory \"{}\": {}",
            str_path, err
        )
    })?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if let Some(p) = path.to_str() {
                return Ok(p.to_string());
            }
        }
    }
    Err(Error::new(format!(
        "error while resolving coretemp path: no directory found under \"{}\"",
        str_path
    )))
}
