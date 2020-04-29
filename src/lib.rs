// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod cpu;
mod error;
mod nl_data;
mod pulse;
mod wireless;
use chrono::prelude::*;
use cpu::Cpu;
use error::Error;
use nl_data::State as WlState;
use pulse::{Pulse, PulseData};
use regex::Regex;
use std::convert::TryFrom;
use std::fs;
use std::time::Duration;
use wireless::Wireless;

const PROC_STAT: &'static str = "/proc/stat";
const PROC_MEMINFO: &'static str = "/proc/meminfo";
const ENERGY_NOW: &'static str = "/sys/class/power_supply/BAT0/energy_now";
const POWER_STATUS: &'static str = "/sys/class/power_supply/BAT0/status";
const ENERGY_FULL_DESIGN: &'static str = "/sys/class/power_supply/BAT0/energy_full_design";
const CORETEMP_PATH: &'static str = "/sys/devices/platform/coretemp.0/hwmon";
const BACKLIGHT_PATH: &'static str =
    "/sys/devices/pci0000:00/0000:00:02.0/drm/card0/card0-eDP-1/intel_backlight";
const DEFAULT_FONT: &'static str = "+@fn=0;";
const ICON_FONT: &'static str = "+@fn=1;";
const DEFAULT_COLOR: &'static str = "+@fg=0;";
const RED: &'static str = "+@fg=1;";
const GREEN: &'static str = "+@fg=2;";
const CPU_RATE: Duration = Duration::from_millis(500);
const WIRELESS_RATE: Duration = Duration::from_millis(500);
const PULSE_RATE: Duration = Duration::from_millis(16);
const SINK_INDEX: u32 = 0;
const SOURCE_INDEX: u32 = 1;

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

pub struct Bar<'a> {
    default_font: &'a str,
    icon: &'a str,
    default_color: &'a str,
    red: &'a str,
    green: &'a str,
    prev_idle: i32,
    prev_total: i32,
    prev_usage: Option<i32>,
    coretemp_path: String,
    pulse: Pulse,
    cpu: Cpu,
    prev_sink: Option<PulseData>,
    prev_source: Option<PulseData>,
    wireless: Wireless,
    prev_wireless: Option<WlState>,
    mem_regex: MemRegex,
}

impl<'a> Bar<'a> {
    pub fn new() -> Result<Self, Error> {
        let path = find_temp_dir(CORETEMP_PATH)?;
        Ok(Bar {
            default_font: DEFAULT_FONT,
            icon: ICON_FONT,
            default_color: DEFAULT_COLOR,
            red: RED,
            green: GREEN,
            prev_idle: 0,
            prev_total: 0,
            coretemp_path: path,
            pulse: Pulse::new(PULSE_RATE, SINK_INDEX, SOURCE_INDEX),
            prev_sink: None,
            prev_source: None,
            prev_usage: None,
            cpu: Cpu::new(CPU_RATE, PROC_STAT),
            wireless: Wireless::new(WIRELESS_RATE),
            prev_wireless: None,
            mem_regex: MemRegex::new(),
        })
    }

    fn sound(&mut self) -> Result<String, Error> {
        let data = self.pulse.output_data();
        if data.is_some() {
            self.prev_sink = data;
        }
        let icon;
        let mut color = self.default_color;
        if let Some(info) = self.prev_sink {
            if info.1 {
                icon = "󰸈";
            } else {
                icon = match info.0 {
                    0..=9 => "󰕿",
                    10..=40 => "󰖀",
                    _ => "󰕾",
                }
            }
            if info.0 > 150 {
                color = self.red;
            }
            Ok(format!(
                "{:3}% {}{}{}{}{}",
                info.0, color, self.icon, icon, self.default_font, self.default_color
            ))
        } else {
            icon = "󰖁";
            Ok(format!("     {}{}{}", self.icon, icon, self.default_font))
        }
    }

    fn mic(&mut self) -> Result<String, Error> {
        let data = self.pulse.input_data();
        if data.is_some() {
            self.prev_source = data;
        }
        let icon;
        let mut color = self.default_color;
        if let Some(info) = self.prev_source {
            if info.1 {
                icon = "󰍭";
            } else {
                icon = "󰍬";
            }
            if info.0 > 150 {
                color = self.red;
            }
            Ok(format!(
                "{:3}% {}{}{}{}{}",
                info.0, color, self.icon, icon, self.default_font, self.default_color
            ))
        } else {
            icon = "󰍮";
            Ok(format!("     {}{}{}", self.icon, icon, self.default_font))
        }
    }

    fn battery(&self) -> Result<String, Error> {
        let energy_full_design = read_and_parse(ENERGY_FULL_DESIGN)?;
        let energy_now = read_and_parse(ENERGY_NOW)?;
        let status = read_and_trim(POWER_STATUS)?;
        let capacity = energy_full_design as u64;
        let energy = energy_now as u64;
        let battery_level = u32::try_from(100_u64 * energy / capacity)?;
        let mut color = match battery_level {
            0..=10 => {
                if status == "Discharging" {
                    self.red
                } else {
                    self.default_color
                }
            }
            _ => self.default_color,
        };
        if status == "Full" {
            color = self.green
        }
        Ok(format!(
            "{:3}% {}{}{}{}{}",
            battery_level,
            color,
            self.icon,
            get_battery_icon(&status, battery_level),
            self.default_font,
            self.default_color
        ))
    }

    fn cpu(&mut self) -> Result<String, Error> {
        let mut current_usg = 0;
        if let Some(data) = self.cpu.data() {
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
        let mut color = self.default_color;
        if current_usg >= 90 {
            color = self.red;
        }
        Ok(format!(
            "{:3}% {}{}󰻠{}{}",
            current_usg, color, self.icon, self.default_font, self.default_color
        ))
    }

    fn core_temperature(&self) -> Result<String, Error> {
        let core_1 = read_and_parse(&format!("{}/temp2_input", self.coretemp_path))?;
        let core_2 = read_and_parse(&format!("{}/temp3_input", self.coretemp_path))?;
        let core_3 = read_and_parse(&format!("{}/temp4_input", self.coretemp_path))?;
        let core_4 = read_and_parse(&format!("{}/temp5_input", self.coretemp_path))?;
        let average =
            (((core_1 + core_2 + core_3 + core_4) as f32 / 4_f32) / 1000_f32).round() as i32;
        let mut color = self.default_color;
        let icon = match average {
            0..=49 => "󱃃",
            50..=69 => "󰔏",
            70..=100 => "󱃂",
            _ => "󰸁",
        };
        if average > 75 {
            color = self.red;
        }
        Ok(format!(
            "{:3}° {}{}{}{}{}",
            average, color, self.icon, icon, self.default_font, self.default_color
        ))
    }

    fn memory(&self) -> Result<String, Error> {
        let meminfo = read_and_trim(PROC_MEMINFO)?;
        let total = find_meminfo(
            &self.mem_regex.total,
            &meminfo,
            &format!("MemTotal not found in \"{}\"", PROC_MEMINFO),
        )?;
        let free = find_meminfo(
            &self.mem_regex.free,
            &meminfo,
            &format!("MemFree not found in \"{}\"", PROC_MEMINFO),
        )?;

        let buffers = find_meminfo(
            &self.mem_regex.buffers,
            &meminfo,
            &format!("Buffers not found in \"{}\"", PROC_MEMINFO),
        )?;
        let cached = find_meminfo(
            &self.mem_regex.cached,
            &meminfo,
            &format!("Cached not found in \"{}\"", PROC_MEMINFO),
        )?;
        let s_reclaimable = find_meminfo(
            &self.mem_regex.s_reclaimable,
            &meminfo,
            &format!("SReclaimable not found in \"{}\"", PROC_MEMINFO),
        )?;
        let used = total - free - buffers - cached - s_reclaimable;
        let percentage = (used as f64 * 100_f64 / total as f64).round() as i32;
        let total_go = (1024_f64 * (total as f64)) / 1_000_000_000_f64;
        let used_go = 1024_f64 * (used as f64) / 1_000_000_000_f64;
        let mut color = self.default_color;
        if percentage > 90 {
            color = self.red;
        }
        Ok(format!(
            "{}{:.1}{}/{:.1}Go󰍛",
            color, used_go, self.default_color, total_go
        ))
    }

    fn wireless(&mut self) -> String {
        if let Some(state) = self.wireless.data() {
            self.prev_wireless = Some(state);
        }
        let icon = if let Some(state) = &self.prev_wireless {
            if let WlState::Connected(data) = state {
                if let Some(strength) = data.signal {
                    match strength {
                        0 => "󰤯",
                        1..=25 => "󰤟",
                        26..=50 => "󰤢",
                        51..=75 => "󰤥",
                        _ => "󰤨",
                    }
                } else {
                    "󰤫"
                }
            } else {
                "󰤮"
            }
        } else {
            "󰤫"
        };
        format!("{}{}{}", self.icon, icon, self.default_font)
    }

    fn brightness(&self) -> Result<String, Error> {
        let brightness = read_and_parse(&format!("{}/actual_brightness", BACKLIGHT_PATH))?;
        let max_brightness = read_and_parse(&format!("{}/max_brightness", BACKLIGHT_PATH))?;
        let percentage = 100 * brightness / max_brightness;
        Ok(format!(
            "{:3}% {}󰃟{}",
            percentage, self.icon, self.default_font
        ))
    }

    pub fn update(&mut self) -> Result<(), Error> {
        let date_time = date_time();
        let battery = self.battery()?;
        let brightness = self.brightness()?;
        let cpu = self.cpu()?;
        let temperature = self.core_temperature()?;
        let sound = self.sound()?;
        let mic = self.mic()?;
        let wireless = self.wireless();
        let memory = self.memory()?;
        println!(
            "{}  {}  {}  {}  {}  {}  {}  {}   {}",
            memory, cpu, temperature, brightness, mic, sound, wireless, battery, date_time
        );
        Ok(())
    }
}

fn read_and_trim<'a>(file: &'a str) -> Result<String, Error> {
    let content = fs::read_to_string(file)
        .map_err(|err| format!("error while reading the file \"{}\": {}", file, err))?;
    Ok(content.trim().to_string())
}

fn read_and_parse<'a>(file: &'a str) -> Result<i32, Error> {
    let content = read_and_trim(file)?;
    let data = content
        .parse::<i32>()
        .map_err(|err| format!("error while parsing the file \"{}\": {}", file, err))?;
    Ok(data)
}

fn find_temp_dir<'a>(str_path: &'a str) -> Result<String, Error> {
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

fn date_time() -> String {
    let now = Local::now();
    now.format("%a. %-e %B %Y, %-kh%M").to_string()
}

fn get_battery_icon<'a>(state: &'a str, level: u32) -> &'static str {
    match state {
        "Full" => "󰁹",
        "Discharging" => match level {
            0..=9 => "󰂎",
            10..=19 => "󰁺",
            20..=29 => "󰁻",
            30..=39 => "󰁼",
            40..=49 => "󰁽",
            50..=59 => "󰁾",
            60..=69 => "󰁿",
            70..=79 => "󰂀",
            80..=89 => "󰂁",
            90..=99 => "󰂂",
            100 => "󰁹",
            _ => "󱃍",
        },
        "Charging" => match level {
            0..=9 => "󰢟",
            10..=19 => "󰢜",
            20..=29 => "󰂆",
            30..=39 => "󰂇",
            40..=49 => "󰂈",
            50..=59 => "󰢝",
            60..=69 => "󰂉",
            70..=79 => "󰢞",
            80..=89 => "󰂊",
            90..=99 => "󰂋",
            100 => "󰂅",
            _ => "󱃍",
        },
        _ => "󱃍",
    }
}
