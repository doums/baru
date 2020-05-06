// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::{BarModule, Config};
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const PROC_STAT: &'static str = "/proc/stat";
const TICK_RATE: Duration = Duration::from_millis(500);

pub struct CpuData(pub i32, pub i32);

pub struct Cpu<'a> {
    handle: JoinHandle<Result<(), Error>>,
    receiver: Receiver<CpuData>,
    prev_idle: i32,
    prev_total: i32,
    prev_usage: Option<i32>,
    config: &'a Config,
}

impl<'a> Cpu<'a> {
    pub fn with_config(config: &'a Config) -> Self {
        let (tx, rx) = mpsc::channel();
        let file = match &config.proc_stat {
            Some(val) => val.clone(),
            None => PROC_STAT.to_string(),
        };
        let tick = match &config.cpu_tick {
            Some(ms) => Duration::from_millis(*ms as u64),
            None => TICK_RATE,
        };
        let handle = thread::spawn(move || -> Result<(), Error> {
            run(tick, file, tx)?;
            Ok(())
        });
        Cpu {
            config,
            handle,
            receiver: rx,
            prev_idle: 0,
            prev_total: 0,
            prev_usage: None,
        }
    }

    pub fn data(&self) -> Option<CpuData> {
        self.receiver.try_iter().last()
    }
}

impl<'a> BarModule for Cpu<'a> {
    fn markup(&self) -> char {
        'c'
    }

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
        if current_usg >= 90 {
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
