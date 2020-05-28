// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::{read_and_trim, BarModule, Config as MainConfig};
use regex::Regex;
use serde::{Deserialize, Serialize};

const MEMINFO: &'static str = "/proc/meminfo";
const DISPLAY: Display = Display::Percentage;
const HIGH_LEVEL: u32 = 90;

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
enum Display {
    Go,
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
            if let Some(v) = &c.display {
                display = *v;
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
        let total = find_meminfo(
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
        let used = total - free - buffers - cached - s_reclaimable;
        let percentage = (used as f64 * 100_f64 / total as f64).round() as i32;
        let total_go = (1024_f64 * (total as f64)) / 1_000_000_000_f64;
        let used_go = 1024_f64 * (used as f64) / 1_000_000_000_f64;
        let mut color = &self.config.default_color;
        if percentage > self.high_level as i32 {
            color = &self.config.red;
        }
        if let Display::Go = self.display {
            Ok(format!(
                "{:4.1}/{:4.1}Go{}{}󰍛{}{}",
                used_go,
                total_go,
                color,
                self.config.icon_font,
                self.config.default_font,
                self.config.default_color
            ))
        } else {
            Ok(format!(
                "{:3}%{}{}󰍛{}{}",
                percentage,
                color,
                self.config.icon_font,
                self.config.default_font,
                self.config.default_color
            ))
        }
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
