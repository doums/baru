// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::module::{Bar, RunPtr};
use crate::util::read_and_parse;
use crate::{Config as MainConfig, ModuleMsg};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};
use std::{fs, io};
use tracing::{debug, instrument};

const PLACEHOLDER: &str = "-";
const CORETEMP: &str = "/sys/devices/platform/coretemp.0/hwmon";
const HIGH_LEVEL: u32 = 75;
const INPUT: u32 = 1;
const TICK_RATE: Duration = Duration::from_millis(50);
const LABEL: &str = "tem";
const HIGH_LABEL: &str = "!te";
const FORMAT: &str = "%l:%v";

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
enum CoreInputs {
    Single(u32),
    Range(String),
    List(Vec<u32>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    coretemp: Option<String>,
    high_level: Option<u32>,
    core_inputs: Option<CoreInputs>,
    tick: Option<u32>,
    placeholder: Option<String>,
    label: Option<String>,
    high_label: Option<String>,
    format: Option<String>,
}

#[derive(Debug)]
pub struct InternalConfig<'a> {
    coretemp: &'a str,
    high_level: u32,
    tick: Duration,
    inputs: Vec<u32>,
    label: &'a str,
    high_label: &'a str,
}

impl<'a> Default for InternalConfig<'a> {
    fn default() -> Self {
        InternalConfig {
            coretemp: CORETEMP,
            high_level: HIGH_LEVEL,
            tick: TICK_RATE,
            inputs: vec![INPUT],
            label: LABEL,
            high_label: HIGH_LABEL,
        }
    }
}

fn check_input_file(path: &str, n: u32) -> bool {
    fs::metadata(format!("{path}/temp{n}_input"))
        .map(|m| m.is_file())
        .inspect_err(|_e| {
            // TODO log error
        })
        .unwrap_or(false)
}

fn check_dir(path: &str) -> Result<bool, io::Error> {
    let meta = fs::metadata(path)?;
    Ok(meta.is_dir())
}

fn get_inputs(core_inputs: &CoreInputs, temp_dir: &str) -> Option<Vec<u32>> {
    let re = Regex::new(r"^(\d+)\.\.(\d+)$").unwrap();

    match core_inputs {
        CoreInputs::Single(n) => {
            if check_input_file(temp_dir, *n) {
                Some(vec![*n])
            } else {
                None
            }
        }
        CoreInputs::Range(range) => {
            if let Some(captured) = re.captures(range) {
                let start = captured.get(1).unwrap().as_str().parse::<u32>().unwrap();
                let end = captured.get(2).unwrap().as_str().parse::<u32>().unwrap();
                if (start..end).is_empty() {
                    // TODO log error on wrong range values
                    return None;
                }
                let inputs = (start..end + 1)
                    .filter(|i| check_input_file(temp_dir, *i))
                    .collect();
                return Some(inputs);
            }
            // TODO log error wrong range format
            None
        }
        CoreInputs::List(list) => Some(
            list.iter()
                .filter(|i| check_input_file(temp_dir, **i))
                .copied()
                .collect(),
        ),
    }
}

impl<'a> TryFrom<&'a MainConfig> for InternalConfig<'a> {
    type Error = Error;

    fn try_from(config: &'a MainConfig) -> Result<Self, Self::Error> {
        let coretemp = config
            .temperature
            .as_ref()
            .and_then(|c| c.coretemp.as_deref())
            .unwrap_or(CORETEMP);
        check_dir(coretemp)?;
        let temp_dir = find_temp_dir(coretemp)?;

        let internal_cfg = config
            .temperature
            .as_ref()
            .map(|c| {
                let inputs = c
                    .core_inputs
                    .as_ref()
                    .and_then(|i| get_inputs(i, &temp_dir))
                    .map(|mut i| {
                        if i.is_empty() {
                            i.push(INPUT);
                        }
                        i
                    })
                    .unwrap_or(vec![INPUT]);

                InternalConfig {
                    coretemp,
                    high_level: c.high_level.unwrap_or(HIGH_LEVEL),
                    tick: c
                        .tick
                        .map_or(TICK_RATE, |t| Duration::from_millis(t as u64)),
                    inputs,
                    label: c.label.as_deref().unwrap_or(LABEL),
                    high_label: c.high_label.as_deref().unwrap_or(HIGH_LABEL),
                }
            })
            .unwrap_or_default();

        Ok(internal_cfg)
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

#[instrument(skip_all)]
pub fn run(key: char, main_config: MainConfig, tx: Sender<ModuleMsg>) -> Result<(), Error> {
    let config = InternalConfig::try_from(&main_config)?;
    debug!("{:#?}", config);
    let temp_dir = find_temp_dir(config.coretemp)?;
    let mut iteration_start: Instant;
    let mut iteration_end: Duration;
    loop {
        iteration_start = Instant::now();
        let mut inputs = vec![];
        for i in &config.inputs {
            inputs.push(read_and_parse(&format!("{}/temp{}_input", temp_dir, i))?)
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
