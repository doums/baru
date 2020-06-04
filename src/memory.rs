// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::{read_and_trim, BarModule, Config as MainConfig};
use regex::Regex;
use serde::{Deserialize, Serialize};

const MEMINFO: &'static str = "/proc/meminfo";
const DISPLAY: Display = Display::GiB;
const HIGH_LEVEL: u32 = 90;

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
enum Display {
    GB,
    GiB,
    Percentage,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    meminfo: Option<String>,
    high_level: Option<u32>,
    display: Option<Display>,
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

#[derive(Debug)]
pub struct Memory<'a> {
    meminfo: &'a str,
    config: &'a MainConfig,
    mem_regex: MemRegex,
    display: Display,
    high_level: u32,
}

impl<'a> Memory<'a> {
    pub fn with_config(config: &'a MainConfig) -> Self {
        let mut meminfo = MEMINFO;
        let mut high_level = HIGH_LEVEL;
        let mut display = DISPLAY;
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
        };
        Memory {
            meminfo,
            mem_regex: MemRegex::new(),
            config,
            high_level,
            display,
        }
    }
}

impl<'a> BarModule for Memory<'a> {
    fn refresh(&mut self) -> Result<String, Error> {
        let meminfo = read_and_trim(self.meminfo)?;
        let total_kib = find_meminfo(
            &self.mem_regex.total,
            &meminfo,
            &format!("MemTotal not found in \"{}\"", self.meminfo),
        )?;
        let free = find_meminfo(
            &self.mem_regex.free,
            &meminfo,
            &format!("MemFree not found in \"{}\"", self.meminfo),
        )?;

        let buffers = find_meminfo(
            &self.mem_regex.buffers,
            &meminfo,
            &format!("Buffers not found in \"{}\"", self.meminfo),
        )?;
        let cached = find_meminfo(
            &self.mem_regex.cached,
            &meminfo,
            &format!("Cached not found in \"{}\"", self.meminfo),
        )?;
        let s_reclaimable = find_meminfo(
            &self.mem_regex.s_reclaimable,
            &meminfo,
            &format!("SReclaimable not found in \"{}\"", self.meminfo),
        )?;
        let used_kib = total_kib - free - buffers - cached - s_reclaimable;
        let percentage = (used_kib as f64 * 100_f64 / total_kib as f64).round() as i32;
        let mut total = "".to_string();
        let mut used = "".to_string();
        match self.display {
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
        let mut color = &self.config.default_color;
        if percentage > self.high_level as i32 {
            color = &self.config.red;
        }
        match self.display {
            Display::GB | Display::GiB => Ok(format!(
                "{}/{}{}{}󰍛{}{}",
                used,
                total,
                color,
                self.config.icon_font,
                self.config.default_font,
                self.config.default_color
            )),
            _ => Ok(format!(
                "{:3}%{}{}󰍛{}{}",
                percentage,
                color,
                self.config.icon_font,
                self.config.default_font,
                self.config.default_color
            )),
        }
    }
}

fn humanize<'a>(v1: f32, v2: f32, u1: &'a str, u2: &'a str) -> String {
    if v1 >= 1.0 {
        return if v1.fract() == 0.0 {
            format!("{:4.0}{}", v1, u1)
        } else {
            format!("{:4.1}{}", v1, u1)
        };
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
