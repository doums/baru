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
mod wireless;
use cpu::Cpu;
use error::Error;
use mic::Mic;
use module::Module;
use pulse::Pulse;
use serde::{Deserialize, Serialize};
use sound::Sound;
use std::fs;
use wireless::Wireless;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum ModuleConfig {
    DateTime,
    Battery,
    Brightness,
    Cpu,
    Temperature,
    Sound,
    Mic,
    Wireless,
    Memory,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Config {
    bar: String,
    pub tick: Option<u32>,
    default_font: String,
    icon_font: String,
    default_color: String,
    red: String,
    green: String,
    sink: Option<u32>,
    source: Option<u32>,
    cpu_tick: Option<u32>,
    wireless_tick: Option<u32>,
    pulse_tick: Option<u32>,
    proc_stat: Option<String>,
    proc_meminfo: Option<String>,
    energy_now: Option<String>,
    power_status: Option<String>,
    energy_full_design: Option<String>,
    coretemp: Option<String>,
    backlight: Option<String>,
}

trait BarModule {
    fn refresh(&mut self) -> Result<String, Error>;
}

pub struct Bar<'a> {
    modules: Vec<Module<'a>>,
    format: &'a str,
    markup_matches: Vec<MarkupMatch>,
}

#[derive(Debug)]
struct MarkupMatch(char, usize);

impl<'a> Bar<'a> {
    pub fn with_config(config: &'a Config, pulse: &'a Pulse) -> Result<Self, Error> {
        let markup_matches = parse_format(&config.bar);
        println!("{:#?}", markup_matches);
        let mut modules = vec![];
        for module in &markup_matches {
            modules.push(Module::new(module.0, config, pulse)?);
        }
        Ok(Bar {
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
