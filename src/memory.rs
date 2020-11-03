// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::module::{BaruMod, RunPtr};
use crate::pulse::Pulse;
use crate::{read_and_trim, Config as MainConfig, ModuleMsg};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const PLACEHOLDER: &str = "+@fn=1;󰍛+@fn=0;";
const MEMINFO: &str = "/proc/meminfo";
const DISPLAY: Display = Display::GiB;
const HIGH_LEVEL: u32 = 90;
const TICK_RATE: Duration = Duration::from_millis(500);

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
enum Display {
    GB,
    GiB,
    Percentage,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    meminfo: Option<String>,
    high_level: Option<u32>,
    display: Option<Display>,
    tick: Option<u32>,
    placeholder: Option<String>,
}

#[derive(Debug)]
pub struct InternalConfig<'a> {
    meminfo: &'a str,
    high_level: u32,
    display: Display,
    tick: Duration,
}

impl<'a> From<&'a MainConfig> for InternalConfig<'a> {
    fn from(config: &'a MainConfig) -> Self {
        let mut meminfo = MEMINFO;
        let mut high_level = HIGH_LEVEL;
        let mut display = DISPLAY;
        let mut tick = TICK_RATE;
        if let Some(c) = &config.memory {
            if let Some(v) = &c.meminfo {
                meminfo = v;
            }
            if let Some(v) = &c.high_level {
                high_level = *v;
            }
            if let Some(v) = c.display {
                display = v;
            }
            if let Some(t) = c.tick {
                tick = Duration::from_millis(t as u64)
            }
        };
        InternalConfig {
            meminfo,
            high_level,
            display,
            tick,
        }
    }
}

#[derive(Debug)]
pub struct Memory<'a> {
    placeholder: &'a str,
    config: &'a MainConfig,
}

impl<'a> Memory<'a> {
    pub fn with_config(config: &'a MainConfig) -> Self {
        let mut placeholder = PLACEHOLDER;
        if let Some(c) = &config.memory {
            if let Some(p) = &c.placeholder {
                placeholder = p
            }
        }
        Memory {
            placeholder,
            config,
        }
    }
}

impl<'a> BaruMod for Memory<'a> {
    fn run_fn(&self) -> RunPtr {
        run
    }

    fn placeholder(&self) -> &str {
        self.placeholder
    }

    fn name(&self) -> &str {
        "memory"
    }
}

#[derive(Debug)]
struct MemRegex {
    total: Regex,
    free: Regex,
    buffers: Regex,
    cached: Regex,
    s_reclaimable: Regex,
}

impl MemRegex {
    fn new() -> Self {
        MemRegex {
            total: Regex::new(r"(?m)^MemTotal:\s*(\d+)\s*kB$").unwrap(),
            free: Regex::new(r"(?m)^MemFree:\s*(\d+)\s*kB$").unwrap(),
            buffers: Regex::new(r"(?m)^Buffers:\s*(\d+)\s*kB$").unwrap(),
            cached: Regex::new(r"(?m)^Cached:\s*(\d+)\s*kB$").unwrap(),
            s_reclaimable: Regex::new(r"(?m)^SReclaimable:\s*(\d+)\s*kB$").unwrap(),
        }
    }
}

pub fn run(
    key: char,
    main_config: MainConfig,
    _: Arc<Mutex<Pulse>>,
    tx: Sender<ModuleMsg>,
) -> Result<(), Error> {
    let config = InternalConfig::from(&main_config);
    let mem_regex = MemRegex::new();
    loop {
        let meminfo = read_and_trim(config.meminfo)?;
        let total_kib = find_meminfo(
            &mem_regex.total,
            &meminfo,
            &format!("MemTotal not found in \"{}\"", config.meminfo),
        )?;
        let free = find_meminfo(
            &mem_regex.free,
            &meminfo,
            &format!("MemFree not found in \"{}\"", config.meminfo),
        )?;
        let buffers = find_meminfo(
            &mem_regex.buffers,
            &meminfo,
            &format!("Buffers not found in \"{}\"", config.meminfo),
        )?;
        let cached = find_meminfo(
            &mem_regex.cached,
            &meminfo,
            &format!("Cached not found in \"{}\"", config.meminfo),
        )?;
        let s_reclaimable = find_meminfo(
            &mem_regex.s_reclaimable,
            &meminfo,
            &format!("SReclaimable not found in \"{}\"", config.meminfo),
        )?;
        let used_kib = total_kib - free - buffers - cached - s_reclaimable;
        let percentage = (used_kib as f64 * 100_f64 / total_kib as f64).round() as i32;
        let mut total = "".to_string();
        let mut used = "".to_string();
        match config.display {
            Display::GB => {
                let total_go = (1024_f32 * (total_kib as f32)) / 1_000_000_000_f32;
                let total_mo = total_go * 10i32.pow(3) as f32;
                total = humanize(total_go, total_mo, "GB", "MB");
                let used_go = 1024_f32 * (used_kib as f32) / 1_000_000_000_f32;
                let used_mo = used_go * 10i32.pow(3) as f32;
                used = humanize(used_go, used_mo, "GB", "MB");
            }
            Display::GiB => {
                let total_gio = total_kib as f32 / 2i32.pow(20) as f32;
                let total_mio = total_kib as f32 / 2i32.pow(10) as f32;
                total = humanize(total_gio, total_mio, "GiB", "MiB");
                let used_gio = used_kib as f32 / 2i32.pow(20) as f32;
                let used_mio = used_kib as f32 / 2i32.pow(10) as f32;
                used = humanize(used_gio, used_mio, "GiB", "MiB");
            }
            _ => {}
        }
        let mut color = &main_config.default_color;
        if percentage > config.high_level as i32 {
            color = &main_config.red;
        }
        match config.display {
            Display::GB | Display::GiB => tx.send(ModuleMsg(
                key,
                format!(
                    "{}/{}{}{}󰍛{}{}",
                    used,
                    total,
                    color,
                    main_config.icon_font,
                    main_config.default_font,
                    main_config.default_color
                ),
            ))?,
            Display::Percentage => tx.send(ModuleMsg(
                key,
                format!(
                    "{:3}%{}{}󰍛{}{}",
                    percentage,
                    color,
                    main_config.icon_font,
                    main_config.default_font,
                    main_config.default_color
                ),
            ))?,
        };
        thread::sleep(config.tick);
    }
}

fn humanize<'a>(v1: f32, v2: f32, u1: &'a str, u2: &'a str) -> String {
    if v1 >= 1.0 {
        if v1.fract() == 0.0 {
            format!("{:4.0}{}", v1, u1)
        } else {
            format!("{:4.1}{}", v1, u1)
        }
    } else {
        format!("{:4.0}{}", v2, u2)
    }
}

fn find_meminfo<'a>(regex: &Regex, meminfo: &'a str, error: &'a str) -> Result<i32, String> {
    let matched = regex
        .captures(&meminfo)
        .ok_or_else(|| error.to_string())?
        .get(1)
        .ok_or_else(|| error.to_string())?
        .as_str();
    Ok(matched
        .parse::<i32>()
        .map_err(|err| format!("error while parsing meminfo: {}", err))?)
}
