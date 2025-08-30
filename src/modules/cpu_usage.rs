// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::module::{Bar, RunPtr};
use crate::{Config as MainConfig, ModuleMsg};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};
use tracing::{debug, instrument};

const PLACEHOLDER: &str = "-";
const PROC_STAT: &str = "/proc/stat";
const TICK_RATE: Duration = Duration::from_millis(500);
const HIGH_LEVEL: u32 = 90;
const LABEL: &str = "cpu";
const HIGH_LABEL: &str = "!cp";
const FORMAT: &str = "%l:%v";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    tick: Option<u32>,
    high_level: Option<u32>,
    placeholder: Option<String>,
    label: Option<String>,
    high_label: Option<String>,
    format: Option<String>,
}

#[derive(Debug)]
pub struct InternalConfig<'a> {
    proc_stat: &'a str,
    high_level: u32,
    tick: Duration,
    label: &'a str,
    high_label: &'a str,
}

impl<'a> From<&'a MainConfig> for InternalConfig<'a> {
    fn from(config: &'a MainConfig) -> Self {
        let mut tick = TICK_RATE;
        let mut high_level = HIGH_LEVEL;
        let mut label = LABEL;
        let mut high_label = HIGH_LABEL;
        if let Some(c) = &config.cpu_usage {
            if let Some(t) = c.tick {
                tick = Duration::from_millis(t as u64)
            }
            if let Some(c) = c.high_level {
                high_level = c;
            }
            if let Some(v) = &c.label {
                label = v;
            }
            if let Some(v) = &c.high_label {
                high_label = v;
            }
        };
        InternalConfig {
            high_level,
            proc_stat: PROC_STAT,
            tick,
            label,
            high_label,
        }
    }
}

#[derive(Debug)]
pub struct CpuUsage<'a> {
    placeholder: &'a str,
    format: &'a str,
}

impl<'a> CpuUsage<'a> {
    pub fn with_config(config: &'a MainConfig) -> Self {
        let mut placeholder = PLACEHOLDER;
        let mut format = FORMAT;
        if let Some(c) = &config.cpu_usage {
            if let Some(p) = &c.placeholder {
                placeholder = p
            }
            if let Some(v) = &c.format {
                format = v;
            }
        }
        CpuUsage {
            placeholder,
            format,
        }
    }
}

impl<'a> Bar for CpuUsage<'a> {
    fn name(&self) -> &str {
        "cpu_usage"
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
pub fn run(
    running: &AtomicBool,
    key: char,
    main_config: MainConfig,
    tx: Sender<ModuleMsg>,
) -> Result<(), Error> {
    let config = InternalConfig::from(&main_config);
    debug!("{:#?}", config);
    let mut prev_idle = 0;
    let mut prev_total = 0;
    let mut iteration_start: Instant;
    let mut iteration_end: Duration;
    while running.load(Ordering::Relaxed) {
        iteration_start = Instant::now();
        let proc_stat = File::open(config.proc_stat)?;
        let mut reader = BufReader::new(proc_stat);
        let mut buf = String::new();
        reader.read_line(&mut buf)?;
        let mut data = buf.split_whitespace();
        data.next();
        let times: Vec<i32> = data
            .map(|n| {
                n.parse::<i32>().unwrap_or_else(|_| {
                    panic!("error while parsing the file \"{}\"", config.proc_stat)
                })
            })
            .collect();
        let idle = times[3] + times[4];
        let total = times.iter().sum();
        let diff_total = total - prev_total;
        let diff_idle = idle - prev_idle;
        let usage = (100_f32 * (diff_total - diff_idle) as f32 / diff_total as f32).round() as i32;
        prev_total = total;
        prev_idle = idle;
        let mut label = config.label;
        if usage >= config.high_level as i32 {
            label = config.high_label;
        }
        tx.send(ModuleMsg(
            key,
            Some(format!("{usage:3}%")),
            Some(label.to_string()),
        ))?;
        iteration_end = iteration_start.elapsed();
        if iteration_end < config.tick {
            thread::sleep(config.tick - iteration_end);
        }
    }
    Ok(())
}
