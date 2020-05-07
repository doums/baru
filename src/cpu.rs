// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::{BarModule, Config as MainConfig};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const PROC_STAT: &'static str = "/proc/stat";
const TICK_RATE: Duration = Duration::from_millis(500);
const HIGH_LEVEL: u32 = 90;

pub struct CpuData(pub i32, pub i32);

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    tick: Option<u32>,
    proc_stat: Option<String>,
    high_level: Option<u32>,
}

#[derive(Debug)]
pub struct Cpu<'a> {
    handle: JoinHandle<Result<(), Error>>,
    receiver: Receiver<CpuData>,
    prev_idle: i32,
    prev_total: i32,
    prev_usage: Option<i32>,
    config: &'a MainConfig,
    high_level: u32,
}

impl<'a> Cpu<'a> {
    pub fn with_config(config: &'a MainConfig) -> Result<Self, Error> {
        let (tx, rx) = mpsc::channel();
        let mut tick = TICK_RATE;
        let mut file = PROC_STAT.to_string();
        let mut high_level = HIGH_LEVEL;
        if let Some(c) = &config.cpu {
            if let Some(f) = &c.proc_stat {
                file = f.clone();
            }
            if let Some(t) = c.tick {
                tick = Duration::from_millis(t as u64)
            }
            if let Some(c) = c.high_level {
                high_level = c;
            }
        };
        let builder = thread::Builder::new().name("cpu_mod".into());
        let handle = builder.spawn(move || -> Result<(), Error> {
            run(tick, file, tx)?;
            Ok(())
        })?;
        Ok(Cpu {
            config,
            handle,
            receiver: rx,
            prev_idle: 0,
            prev_total: 0,
            prev_usage: None,
            high_level,
        })
    }

    pub fn data(&self) -> Option<CpuData> {
        self.receiver.try_iter().last()
    }
}

impl<'a> BarModule for Cpu<'a> {
    fn refresh(&mut self) -> Result<String, Error> {
        let mut current_usg = 0;
        if let Some(data) = self.data() {
            let diff_total = data.0 - self.prev_total;
            let diff_idle = data.1 - self.prev_idle;
            let usage =
                (100_f32 * (diff_total - diff_idle) as f32 / diff_total as f32).round() as i32;
            self.prev_total = data.0;
            self.prev_idle = data.1;
            self.prev_usage = Some(usage);
            current_usg = usage;
        } else {
            if let Some(usage) = self.prev_usage {
                current_usg = usage;
            }
        }
        let mut color = &self.config.default_color;
        if current_usg >= self.high_level as i32 {
            color = &self.config.red;
        }
        Ok(format!(
            "{:3}% {}{}󰻠{}{}",
            current_usg,
            color,
            self.config.icon_font,
            self.config.default_font,
            self.config.default_color
        ))
    }
}

fn run(tick: Duration, file: String, tx: Sender<CpuData>) -> Result<(), Error> {
    loop {
        let proc_stat = File::open(&file)?;
        let mut reader = BufReader::new(proc_stat);
        let mut buf = String::new();
        reader.read_line(&mut buf)?;
        let mut data = buf.split_whitespace();
        data.next();
        let times: Vec<i32> = data
            .map(|n| {
                n.parse::<i32>()
                    .expect(&format!("error while parsing the file \"{}\"", file))
            })
            .collect();
        let idle = times[3] + times[4];
        let total = times.iter().fold(0, |acc, i| acc + i);
        tx.send(CpuData(total, idle))?;
        thread::sleep(tick);
    }
}
