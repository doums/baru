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
use battery::Battery;
use brightness::Brightness;
use cpu::Cpu;
use date_time::DateTime as MDateTime;
use error::Error;
use memory::Memory;
use mic::Mic;
use module::Module;
use pulse::Pulse;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sound::Sound;
use std::fs;
use temperature::Temperature;
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
    pub modules: Vec<ModuleConfig>,
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
    fn markup(&self) -> char;
}

pub struct Bar<'a> {
    modules: Vec<Module<'a>>,
    format: String,
}

impl<'a> Bar<'a> {
    pub fn with_config(config: &'a Config, pulse: &'a Option<Pulse>) -> Result<Self, Error> {
        let re = Regex::new(r"(?:[^\\]|^)%([abcdmistw])").unwrap();
        let caps = re.captures_iter(&config.bar);
        for cap in caps {
            println!("{:#?}", cap);
        }
        let mut modules = vec![];
        for module in &config.modules {
            match module {
                ModuleConfig::DateTime => modules.push(Module::DateTime(MDateTime::new())),
                ModuleConfig::Battery => {
                    modules.push(Module::Battery(Battery::with_config(config)))
                }
                ModuleConfig::Memory => modules.push(Module::Memory(Memory::with_config(config))),
                ModuleConfig::Brightness => {
                    modules.push(Module::Brightness(Brightness::with_config(config)))
                }
                ModuleConfig::Temperature => {
                    modules.push(Module::Temperature(Temperature::with_config(config)?))
                }
                ModuleConfig::Cpu => modules.push(Module::Cpu(Cpu::with_config(config))),
                ModuleConfig::Sound => modules.push(Module::Sound(Sound::with_config(
                    config,
                    pulse
                        .as_ref()
                        .expect("no Pulse module while creating Sound module"),
                ))),
                ModuleConfig::Mic => modules.push(Module::Mic(Mic::with_config(
                    config,
                    pulse
                        .as_ref()
                        .expect("no Pulse module while creating Sound module"),
                ))),
                ModuleConfig::Wireless => {
                    modules.push(Module::Wireless(Wireless::with_config(config)))
                }
            }
        }
        Ok(Bar {
            modules,
            format: config.bar.to_string(),
        })
    }

    pub fn update(&mut self) -> Result<(), Error> {
        // println!(
        // "{}  {}  {}  {}  {}  {}  {}  {}   {}",
        // memory, cpu, temperature, brightness, mic, sound, wireless, battery, date_time
        // );
        let test = r"{}  {}  {}   {}";
        // println!("{}", test, "mic", "");
        // for module in &mut self.modules {
        // println!("{}", module.refresh()?);
        // }
        // println!("");
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
