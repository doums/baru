// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::module::{Bar, RunPtr};
use crate::Pulse;
use crate::{read_and_parse, Config as MainConfig, ModuleMsg};
use serde::{Deserialize, Serialize};
use std::fs::{read_dir, DirEntry};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use std::{convert::TryFrom, path::Path};

const PLACEHOLDER: &str = "-";
const TICK_RATE: Duration = Duration::from_millis(100);
const HIGH_LEVEL: u32 = 80;
const LABEL: &str = "fre";
const HIGH_LABEL: &str = "!fr";
const FORMAT: &str = "%l:%v";
const SYSFS_CPUFREQ: &str = "/sys/devices/system/cpu/cpufreq";
const CPUINFO_MAX_FREQ: &str = "cpuinfo_max_freq";
const SCALING_MAX_FREQ: &str = "scaling_max_freq";
const CPUINFO_CUR_FREQ: &str = "cpuinfo_cur_freq";
const SCALING_CUR_FREQ: &str = "scaling_cur_freq";
const UNIT: Unit = Unit::Smart;
const MAX_FREQ: bool = false;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    tick: Option<u32>,
    unit: Option<Unit>,
    max_freq: Option<bool>,
    high_level: Option<u32>,
    placeholder: Option<String>,
    label: Option<String>,
    high_label: Option<String>,
    format: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
enum Unit {
    MHz,
    GHz,
    Smart,
}

#[derive(Debug)]
pub struct InternalConfig<'a> {
    high_level: u32,
    tick: Duration,
    max_freq: f32,
    unit: Unit,
    show_max_freq: bool,
    label: &'a str,
    high_label: &'a str,
    cur_freq_attribute: &'a str,
}

impl<'a> TryFrom<&'a MainConfig> for InternalConfig<'a> {
    type Error = Error;

    fn try_from(config: &'a MainConfig) -> Result<Self, Self::Error> {
        let mut tick = TICK_RATE;
        let mut show_max_freq = MAX_FREQ;
        let mut unit = UNIT;
        let mut high_level = HIGH_LEVEL;
        let mut label = LABEL;
        let mut high_label = HIGH_LABEL;
        if let Some(c) = &config.cpu_freq {
            if let Some(t) = c.tick {
                tick = Duration::from_millis(t as u64)
            }
            if let Some(c) = c.high_level {
                high_level = c;
            }
            if let Some(v) = c.max_freq {
                show_max_freq = v;
            }
            if let Some(v) = c.unit {
                unit = v;
            }
            if let Some(v) = &c.label {
                label = v;
            }
            if let Some(v) = &c.high_label {
                high_label = v;
            }
        };
        let policy_path = format!("{}/policy0", SYSFS_CPUFREQ);
        let entries: Vec<DirEntry> = read_dir(Path::new(&policy_path))?
            .filter_map(|entry| entry.ok())
            .collect();
        let cpuinfo_max_freq = entries
            .iter()
            .find(|&entry| entry.file_name().to_str() == Some(CPUINFO_MAX_FREQ));
        let scaling_max_freq = entries
            .iter()
            .find(|&entry| entry.file_name().to_str() == Some(SCALING_MAX_FREQ));
        let cpuinfo_cur_freq = entries
            .iter()
            .any(|entry| entry.file_name().to_str() == Some(CPUINFO_CUR_FREQ));
        let scaling_cur_freq = entries
            .iter()
            .any(|entry| entry.file_name().to_str() == Some(SCALING_CUR_FREQ));
        if !cpuinfo_cur_freq && !scaling_cur_freq {
            return Err(Error::new("fail to find current cpu freq"));
        }
        let cur_freq_attribute = if scaling_cur_freq {
            SCALING_CUR_FREQ
        } else {
            CPUINFO_CUR_FREQ
        };
        let max_freq = if let Some(entry) = scaling_max_freq {
            read_and_parse(entry.path().to_str().unwrap())? as u32
        } else if let Some(entry) = cpuinfo_max_freq {
            read_and_parse(entry.path().to_str().unwrap())? as u32
        } else {
            return Err(Error::new("fail to find max cpu freq"));
        };
        Ok(InternalConfig {
            high_level,
            tick,
            show_max_freq,
            max_freq: (max_freq / 1000) as f32,
            unit,
            label,
            high_label,
            cur_freq_attribute,
        })
    }
}

#[derive(Debug)]
pub struct CpuFreq<'a> {
    placeholder: &'a str,
    config: &'a MainConfig,
    format: &'a str,
}

impl<'a> CpuFreq<'a> {
    pub fn with_config(config: &'a MainConfig) -> Self {
        let mut placeholder = PLACEHOLDER;
        let mut format = FORMAT;
        if let Some(c) = &config.cpu_freq {
            if let Some(p) = &c.placeholder {
                placeholder = p
            }
            if let Some(v) = &c.format {
                format = v;
            }
        }
        CpuFreq {
            placeholder,
            config,
            format,
        }
    }
}

impl<'a> Bar for CpuFreq<'a> {
    fn name(&self) -> &str {
        "cpu_freq"
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
        let freqs: Vec<f32> = read_dir(Path::new(SYSFS_CPUFREQ))?
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                if let Some(value) = entry.path().to_str() {
                    read_and_parse(format!("{}/{}", value, config.cur_freq_attribute).as_str()).ok()
                } else {
                    None
                }
            })
            .map(|freq| freq as f32 / 1000f32)
            .collect();
        let avg = freqs.iter().sum::<f32>() / freqs.len() as f32;
        let value = match config.show_max_freq {
            true => format!(
                "{}/{}",
                humanize(avg, config.unit),
                humanize(config.max_freq as f32, config.unit)
            ),
            false => humanize(avg, config.unit),
        };
        let percentage = ((avg * 100f32) / config.max_freq).round() as u32;
        let label = if percentage >= config.high_level {
            config.high_label
        } else {
            config.label
        };
        tx.send(ModuleMsg(key, Some(value), Some(label.to_string())))?;
        iteration_end = iteration_start.elapsed();
        if iteration_end < config.tick {
            thread::sleep(config.tick - iteration_end);
        }
    }
}

fn humanize(average: f32, unit: Unit) -> String {
    match unit {
        Unit::GHz => format!("{:3.1}GHz", average / 1000f32),
        Unit::MHz => format!("{:4}MHz", average.round() as u32),
        Unit::Smart => {
            let rounded = average.round();
            if rounded < 1000f32 {
                format!("{:3}MHz", rounded as u32)
            } else {
                format!("{:3.1}GHz", average / 1000f32)
            }
        }
    }
}
