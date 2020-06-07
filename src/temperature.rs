// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::module::BaruMod;
use crate::pulse::Pulse;
use crate::{read_and_parse, Config as MainConfig};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::fs;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const PLACEHOLDER: &str = "+@fn=1;󱃃+@fn=0;";
const CORETEMP: &'static str = "/sys/devices/platform/coretemp.0/hwmon";
const HIGH_LEVEL: u32 = 75;
const INPUT: u32 = 1;
const TICK_RATE: Duration = Duration::from_millis(50);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    coretemp: Option<String>,
    high_level: Option<u32>,
    core_inputs: Option<String>,
    tick: Option<u32>,
    placeholder: Option<String>,
}

#[derive(Debug)]
pub struct InternalConfig {
    coretemp: String,
    high_level: u32,
    tick: Duration,
    inputs: Vec<u32>,
}

impl<'a> TryFrom<&'a MainConfig> for InternalConfig {
    type Error = Error;

    fn try_from(config: &'a MainConfig) -> Result<Self, Self::Error> {
        let mut tick = TICK_RATE;
        let mut coretemp = CORETEMP;
        let mut high_level = HIGH_LEVEL;
        let mut inputs = vec![];
        let error = "error when parsing temperature config, wrong core_inputs option, a digit or an inclusive range (eg. 2..4) expected";
        let re = Regex::new(r"^(\d+)\.\.(\d+)$").unwrap();
        if let Some(c) = &config.temperature {
            if let Some(v) = &c.coretemp {
                coretemp = &v;
            }
            if let Some(v) = c.high_level {
                high_level = v;
            }
            if let Some(t) = c.tick {
                tick = Duration::from_millis(t as u64)
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
        Ok(InternalConfig {
            coretemp: find_temp_dir(coretemp)?,
            high_level,
            inputs,
            tick,
        })
    }
}

#[derive(Debug)]
pub struct Temperature<'a> {
    placeholder: &'a str,
    config: &'a MainConfig,
}

impl<'a> Temperature<'a> {
    pub fn with_config(config: &'a MainConfig) -> Self {
        let mut placeholder = PLACEHOLDER;
        if let Some(c) = &config.temperature {
            if let Some(p) = &c.placeholder {
                placeholder = p
            }
        }
        Temperature {
            placeholder,
            config,
        }
    }
}

impl<'a> BaruMod for Temperature<'a> {
    fn run_fn(&self) -> fn(MainConfig, Arc<Mutex<Pulse>>, Sender<String>) -> Result<(), Error> {
        run
    }

    fn placeholder(&self) -> &str {
        self.placeholder
    }
}

pub fn run(main_config: MainConfig, _: Arc<Mutex<Pulse>>, tx: Sender<String>) -> Result<(), Error> {
    let config = InternalConfig::try_from(&main_config)?;
    loop {
        let mut inputs = vec![];
        for i in &config.inputs {
            inputs.push(read_and_parse(&format!(
                "{}/temp{}_input",
                config.coretemp, i
            ))?)
        }
        let sum = inputs.iter().fold(0, |acc, x| acc + x);
        let average = ((sum as f32 / inputs.len() as f32) / 1000_f32).round() as i32;
        let mut color = &main_config.default_color;
        let icon = match average {
            0..=49 => "󱃃",
            50..=69 => "󰔏",
            70..=100 => "󱃂",
            _ => "󰸁",
        };
        if average >= config.high_level as i32 {
            color = &main_config.red;
        }
        tx.send(format!(
            "{:3}°{}{}{}{}{}",
            average,
            color,
            main_config.icon_font,
            icon,
            main_config.default_font,
            main_config.default_color
        ))?;
        thread::sleep(config.tick);
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
