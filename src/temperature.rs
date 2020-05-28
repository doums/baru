// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::{read_and_parse, BarModule, Config as MainConfig};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;

const CORETEMP: &'static str = "/sys/devices/platform/coretemp.0/hwmon";
const HIGH_LEVEL: u32 = 75;
const INPUT: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    coretemp: Option<String>,
    high_level: Option<u32>,
    core_inputs: Option<String>,
}

#[derive(Debug)]
pub struct Temperature<'a> {
    coretemp: String,
    config: &'a MainConfig,
    high_level: u32,
    inputs: Vec<u32>,
}

impl<'a> Temperature<'a> {
    pub fn with_config(config: &'a MainConfig) -> Result<Self, Error> {
        let mut path = CORETEMP;
        let mut high_level = HIGH_LEVEL;
        let mut inputs = vec![];
        let error = "error when parsing temperature config, wrong core_inputs option, a digit or an inclusive range (eg. 2..4) expected";
        let re = Regex::new(r"^(\d+)\.\.(\d+)$").unwrap();
        if let Some(c) = &config.temperature {
            if let Some(v) = &c.coretemp {
                path = &v;
            }
            if let Some(v) = c.high_level {
                high_level = v;
            }
            if let Some(i) = &c.core_inputs {
                if let Ok(v) = i.parse::<u32>() {
                    inputs.push(v);
                } else {
                    if let Some(caps) = re.captures(i) {
                        let start = caps.get(1).unwrap().as_str().parse::<u32>().unwrap();
                        let end = caps.get(2).unwrap().as_str().parse::<u32>().unwrap();
                        if start > end {
                            return Err(Error::new(error));
                        }
                        for i in start..end + 1 {
                            inputs.push(i)
                        }
                    } else {
                        return Err(Error::new(error));
                    }
                }
            }
        }
        if inputs.is_empty() {
            inputs.push(INPUT);
        }
        Ok(Temperature {
            coretemp: find_temp_dir(path)?,
            config,
            high_level,
            inputs,
        })
    }
}

impl<'a> BarModule for Temperature<'a> {
    fn refresh(&mut self) -> Result<String, Error> {
        let mut inputs = vec![];
        for i in &self.inputs {
            inputs.push(read_and_parse(&format!(
                "{}/temp{}_input",
                self.coretemp, i
            ))?)
        }
        let sum = inputs.iter().fold(0, |acc, x| acc + x);
        let average = ((sum as f32 / inputs.len() as f32) / 1000_f32).round() as i32;
        let mut color = &self.config.default_color;
        let icon = match average {
            0..=49 => "󱃃",
            50..=69 => "󰔏",
            70..=100 => "󱃂",
            _ => "󰸁",
        };
        if average >= self.high_level as i32 {
            color = &self.config.red;
        }
        Ok(format!(
            "{:3}°{}{}{}{}{}",
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
