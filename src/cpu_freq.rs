// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::module::{Bar, RunPtr};
use crate::Pulse;
use crate::{read_and_parse, Config as MainConfig, ModuleMsg};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const PLACEHOLDER: &str = "-";
const TICK_RATE: Duration = Duration::from_millis(100);
const HIGH_LEVEL: u32 = 80;
const LABEL: &str = "fre";
const HIGH_LABEL: &str = "!fr";
const FORMAT: &str = "%l:%v";
const CPU_FREQ: &str = "/sys/devices/system/cpu/cpu0/cpufreq";
const CPU_MAX_FREQ: &str = "cpuinfo_max_freq";
const CPU_INFO: &str = "/proc/cpuinfo";
const MHZ_KEY: &str = "cpu MHz";
const UNIT: Unit = Unit::Smart;
const MAX_FREQ: bool = false;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    tick: Option<u32>,
    cpufreq_path: Option<String>,
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
}

impl<'a> TryFrom<&'a MainConfig> for InternalConfig<'a> {
    type Error = Error;

    fn try_from(config: &'a MainConfig) -> Result<Self, Self::Error> {
        let mut tick = TICK_RATE;
        let mut cpu_freq_path = CPU_FREQ;
        let mut show_max_freq = MAX_FREQ;
        let mut unit = UNIT;
        let mut high_level = HIGH_LEVEL;
        let mut label = LABEL;
        let mut high_label = HIGH_LABEL;
        if let Some(c) = &config.cpu_freq {
            if let Some(f) = &c.cpufreq_path {
                cpu_freq_path = &f;
            }
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
        let max_freq = read_and_parse(&format!("{}/{}", cpu_freq_path, CPU_MAX_FREQ))? as u32;
        Ok(InternalConfig {
            high_level,
            tick,
            show_max_freq,
            max_freq: (max_freq / 1000) as f32,
            unit,
            label,
            high_label,
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
        let mut freqs = vec![];
        let file = File::open(&CPU_INFO)?;
        let f = BufReader::new(file);
        for line in f.lines() {
            if let Ok(l) = line {
                if l.starts_with(&MHZ_KEY) {
                    let value = l.split_ascii_whitespace().last();
                    if let Some(v) = value {
                        freqs.push(v.parse::<f32>()?);
                    }
                }
            }
        }
        let mut avg: f32 = freqs.iter().sum();
        avg /= freqs.len() as f32;
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
