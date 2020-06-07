// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod battery;
mod brightness;
mod cpu;
mod date_time;
mod error;
mod memory;
mod mic;
mod module;
mod nl_data;
pub mod pulse;
mod sound;
mod temperature;
mod wired;
mod wireless;
use battery::Config as BatteryConfig;
use brightness::Config as BrightnessConfig;
use cpu::Config as CpuConfig;
use date_time::Config as DateTimeConfig;
use error::Error;
use memory::Config as MemoryConfig;
use mic::Config as MicConfig;
use module::Wrapper;
use pulse::Pulse;
use serde::{Deserialize, Serialize};
use sound::Config as SoundConfig;
use std::fs;
use std::sync::{Arc, Mutex};
use temperature::Config as TemperatureConfig;
use wired::Config as WiredConfig;
use wireless::Config as WirelessConfig;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    bar: String,
    pub tick: Option<u32>,
    default_font: String,
    icon_font: String,
    default_color: String,
    red: String,
    green: String,
    pulse_tick: Option<u32>,
    brightness: Option<BrightnessConfig>,
    battery: Option<BatteryConfig>,
    cpu: Option<CpuConfig>,
    memory: Option<MemoryConfig>,
    mic: Option<MicConfig>,
    sound: Option<SoundConfig>,
    temperature: Option<TemperatureConfig>,
    wireless: Option<WirelessConfig>,
    wired: Option<WiredConfig>,
    date_time: Option<DateTimeConfig>,
}

pub struct Baru<'a> {
    modules: Vec<Wrapper<'a>>,
    format: &'a str,
    markup_matches: Vec<MarkupMatch>,
}

#[derive(Debug)]
struct MarkupMatch(char, usize);

impl<'a> Baru<'a> {
    pub fn with_config(config: &'a Config, pulse: &'a Arc<Mutex<Pulse>>) -> Result<Self, Error> {
        let mut modules = vec![];
        let markup_matches = parse_format(&config.bar);
        for module in &markup_matches {
            modules.push(Wrapper::new(module.0, config, &pulse)?);
        }
        for module in &mut modules {
            module.start()?;
        }
        Ok(Baru {
            modules,
            format: &config.bar,
            markup_matches,
        })
    }

    pub fn update(&mut self) -> Result<(), Error> {
        let mut output = self.format.to_string();
        for (i, v) in self.markup_matches.iter().enumerate().rev() {
            output.replace_range(v.1 - 1..v.1 + 1, &self.modules[i].refresh()?);
        }
        output = output.replace("\\%", "%");
        println!("{}", output);
        Ok(())
    }
}

fn parse_format(format: &str) -> Vec<MarkupMatch> {
    let mut matches = vec![];
    let mut iter = format.char_indices().peekable();
    while let Some((i, c)) = iter.next() {
        if c == '%' && (i == 0 || &format[i - 1..i] != "\\") {
            if let Some(val) = iter.peek() {
                matches.push(MarkupMatch(val.1, val.0));
            }
        }
    }
    matches
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
