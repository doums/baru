// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::module::{Bar, RunPtr};
use crate::pulse::Pulse;
use crate::{read_and_parse, Config as MainConfig, ModuleMsg};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::fs;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const PLACEHOLDER: &str = "-";
const CORETEMP: &str = "/sys/devices/platform/coretemp.0/hwmon";
const HIGH_LEVEL: u32 = 75;
const INPUT: u32 = 1;
const TICK_RATE: Duration = Duration::from_millis(50);
const LABEL: &str = "tem";
const HIGH_LABEL: &str = "!te";
const FORMAT: &str = "%l:%v";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    coretemp: Option<String>,
    high_level: Option<u32>,
    core_inputs: Option<String>,
    tick: Option<u32>,
    placeholder: Option<String>,
    label: Option<String>,
    high_label: Option<String>,
    format: Option<String>,
}

#[derive(Debug)]
pub struct InternalConfig<'a> {
    coretemp: String,
    high_level: u32,
    tick: Duration,
    inputs: Vec<u32>,
    label: &'a str,
    high_label: &'a str,
}

impl<'a> TryFrom<&'a MainConfig> for InternalConfig<'a> {
    type Error = Error;

    fn try_from(config: &'a MainConfig) -> Result<Self, Self::Error> {
        let mut tick = TICK_RATE;
        let mut coretemp = CORETEMP;
        let mut high_level = HIGH_LEVEL;
        let mut inputs = vec![];
        let error = "error when parsing temperature config, wrong core_inputs option, a digit or an inclusive range (eg. 2..4) expected";
        let re = Regex::new(r"^(\d+)\.\.(\d+)$").unwrap();
        let mut label = LABEL;
        let mut high_label = HIGH_LABEL;
        if let Some(c) = &config.temperature {
            if let Some(v) = &c.coretemp {
                coretemp = v;
            }
            if let Some(v) = c.high_level {
                high_level = v;
            }
            if let Some(t) = c.tick {
                tick = Duration::from_millis(t as u64)
            }
            if let Some(v) = &c.label {
                label = v;
            }
            if let Some(v) = &c.high_label {
                high_label = v;
            }
            if let Some(i) = &c.core_inputs {
                if let Ok(v) = i.parse::<u32>() {
                    inputs.push(v);
                } else if let Some(caps) = re.captures(i) {
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
        if inputs.is_empty() {
            inputs.push(INPUT);
        }
        Ok(InternalConfig {
            coretemp: find_temp_dir(coretemp)?,
            high_level,
            inputs,
            tick,
            label,
            high_label,
        })
    }
}

#[derive(Debug)]
pub struct Temperature<'a> {
    placeholder: &'a str,
    format: &'a str,
}

impl<'a> Temperature<'a> {
    pub fn with_config(config: &'a MainConfig) -> Self {
        let mut placeholder = PLACEHOLDER;
        let mut format = FORMAT;
        if let Some(c) = &config.temperature {
            if let Some(p) = &c.placeholder {
                placeholder = p
            }
            if let Some(v) = &c.format {
                format = v;
            }
        }
        Temperature {
            placeholder,
            format,
        }
    }
}

impl<'a> Bar for Temperature<'a> {
    fn name(&self) -> &str {
        "temperature"
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
    let config = InternalConfig::try_from(&main_config)?;
    let mut iteration_start: Instant;
    let mut iteration_end: Duration;
    loop {
        iteration_start = Instant::now();
        let mut inputs = vec![];
        for i in &config.inputs {
            inputs.push(read_and_parse(&format!(
                "{}/temp{}_input",
                config.coretemp, i
            ))?)
        }
        let sum: i32 = inputs.iter().sum();
        let average = ((sum as f32 / inputs.len() as f32) / 1000_f32).round() as i32;
        let mut label = config.label;
        if average >= config.high_level as i32 {
            label = config.high_label;
        }
        tx.send(ModuleMsg(
            key,
            Some(format!("{:3}°", average)),
            Some(label.to_string()),
        ))?;
        iteration_end = iteration_start.elapsed();
        if iteration_end < config.tick {
            thread::sleep(config.tick - iteration_end);
        }
    }
}

fn find_temp_dir(str_path: &str) -> Result<String, Error> {
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
