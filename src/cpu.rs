// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::module::BaruMod;
use crate::Config as MainConfig;
use crate::Pulse;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const PLACEHOLDER: &str = "+@fn=1;󰻠+@fn=0;";
const PROC_STAT: &'static str = "/proc/stat";
const TICK_RATE: Duration = Duration::from_millis(500);
const HIGH_LEVEL: u32 = 90;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    tick: Option<u32>,
    proc_stat: Option<String>,
    high_level: Option<u32>,
    placeholder: Option<String>,
}

#[derive(Debug)]
pub struct InternalConfig<'a> {
    proc_stat: &'a str,
    high_level: u32,
    tick: Duration,
}

impl<'a> From<&'a MainConfig> for InternalConfig<'a> {
    fn from(config: &'a MainConfig) -> Self {
        let mut tick = TICK_RATE;
        let mut proc_stat = PROC_STAT;
        let mut high_level = HIGH_LEVEL;
        if let Some(c) = &config.cpu {
            if let Some(f) = &c.proc_stat {
                proc_stat = &f;
            }
            if let Some(t) = c.tick {
                tick = Duration::from_millis(t as u64)
            }
            if let Some(c) = c.high_level {
                high_level = c;
            }
        };
        InternalConfig {
            high_level,
            proc_stat,
            tick,
        }
    }
}

#[derive(Debug)]
pub struct Cpu<'a> {
    placeholder: &'a str,
    config: &'a MainConfig,
}

impl<'a> Cpu<'a> {
    pub fn with_config(config: &'a MainConfig) -> Self {
        let mut placeholder = PLACEHOLDER;
        if let Some(c) = &config.cpu {
            if let Some(p) = &c.placeholder {
                placeholder = p
            }
        }
        Cpu {
            placeholder,
            config,
        }
    }
}

impl<'a> BaruMod for Cpu<'a> {
    fn run_fn(&self) -> fn(MainConfig, Arc<Mutex<Pulse>>, Sender<String>) -> Result<(), Error> {
        run
    }

    fn placeholder(&self) -> &str {
        self.placeholder
    }
}

pub fn run(main_config: MainConfig, _: Arc<Mutex<Pulse>>, tx: Sender<String>) -> Result<(), Error> {
    let config = InternalConfig::from(&main_config);
    let mut prev_idle = 0;
    let mut prev_total = 0;
    loop {
        let proc_stat = File::open(&config.proc_stat)?;
        let mut reader = BufReader::new(proc_stat);
        let mut buf = String::new();
        reader.read_line(&mut buf)?;
        let mut data = buf.split_whitespace();
        data.next();
        let times: Vec<i32> = data
            .map(|n| {
                n.parse::<i32>().expect(&format!(
                    "error while parsing the file \"{}\"",
                    config.proc_stat
                ))
            })
            .collect();
        let idle = times[3] + times[4];
        let total = times.iter().fold(0, |acc, i| acc + i);
        let diff_total = total - prev_total;
        let diff_idle = idle - prev_idle;
        let usage = (100_f32 * (diff_total - diff_idle) as f32 / diff_total as f32).round() as i32;
        prev_total = total;
        prev_idle = idle;
        let mut color = &main_config.default_color;
        if usage >= config.high_level as i32 {
            color = &main_config.red;
        }
        tx.send(format!(
            "{:3}%{}{}󰻠{}{}",
            usage,
            color,
            &main_config.icon_font,
            &main_config.default_font,
            &main_config.default_color
        ))?;
        thread::sleep(config.tick);
    }
}
