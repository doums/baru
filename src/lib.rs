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
use module::{Bar, ModuleData};
use pulse::Pulse;
use serde::{Deserialize, Serialize};
use sound::Config as SoundConfig;
use std::fs;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use temperature::Config as TemperatureConfig;
use wired::Config as WiredConfig;
use wireless::Config as WirelessConfig;

#[derive(Debug)]
pub struct ModuleMsg(char, Option<String>, Option<String>);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    format: String,
    pub tick: Option<u32>,
    pulse_tick: Option<u32>,
    battery: Option<BatteryConfig>,
    brightness: Option<BrightnessConfig>,
    cpu: Option<CpuConfig>,
    date_time: Option<DateTimeConfig>,
    memory: Option<MemoryConfig>,
    mic: Option<MicConfig>,
    sound: Option<SoundConfig>,
    temperature: Option<TemperatureConfig>,
    wired: Option<WiredConfig>,
    wireless: Option<WirelessConfig>,
}

pub struct Baru<'a> {
    config: &'a Config,
    modules: Vec<ModuleData<'a>>,
    format: &'a str,
    pulse: &'a Arc<Mutex<Pulse>>,
    markup_matches: Vec<MarkupMatch>,
    channel: (Sender<ModuleMsg>, Receiver<ModuleMsg>),
}

#[derive(Debug)]
struct MarkupMatch(char, usize);

impl<'a> Baru<'a> {
    pub fn with_config(config: &'a Config, pulse: &'a Arc<Mutex<Pulse>>) -> Result<Self, Error> {
        let mut modules = vec![];
        let markup_matches = parse_format(&config.format);
        for markup in &markup_matches {
            modules.push(ModuleData::new(markup.0, config)?);
        }
        Ok(Baru {
            config,
            pulse,
            channel: mpsc::channel(),
            modules,
            format: &config.format,
            markup_matches,
        })
    }

    pub fn start(&self) -> Result<(), Error> {
        for data in &self.modules {
            let builder = thread::Builder::new().name(format!("mod_{}", data.module.name()));
            let cloned_m_conf = self.config.clone();
            let tx1 = mpsc::Sender::clone(&self.channel.0);
            let pulse = Arc::clone(self.pulse);
            let run = data.module.run_fn();
            let key = data.key;
            builder.spawn(move || -> Result<(), Error> {
                run(key, cloned_m_conf, pulse, tx1)?;
                Ok(())
            })?;
        }
        Ok(())
    }

    fn module_output(&self, key: char) -> Result<&str, Error> {
        let module = self
            .modules
            .iter()
            .find(|data| data.key == key)
            .ok_or(format!("module for key \"{}\" not found", key))?;
        Ok(module.output())
    }

    pub fn update(&mut self) -> Result<(), Error> {
        let messages: Vec<ModuleMsg> = self.channel.1.try_iter().collect();
        for module in &mut self.modules {
            let mut iter = messages.iter().rev();
            let message = iter.find(|v| v.0 == module.key);
            if let Some(value) = message {
                module.new_data(value.1.as_deref(), value.2.as_deref());
            }
        }
        let mut output = self.format.to_string();
        for v in self.markup_matches.iter().rev() {
            output.replace_range(v.1 - 1..v.1 + 1, self.module_output(v.0)?);
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

fn read_and_trim(file: &str) -> Result<String, Error> {
    let content = fs::read_to_string(file)
        .map_err(|err| format!("error while reading the file \"{}\": {}", file, err))?;
    Ok(content.trim().to_string())
}

fn read_and_parse(file: &str) -> Result<i32, Error> {
    let content = read_and_trim(file)?;
    let data = content
        .parse::<i32>()
        .map_err(|err| format!("error while parsing the file \"{}\": {}", file, err))?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_format() {
        let result = parse_format("");
        assert_eq!(result.is_empty(), true);
    }

    #[test]
    fn parse_one_char() {
        let result = parse_format("a");
        assert_eq!(result.is_empty(), true);
    }

    #[test]
    fn parse_one_percent() {
        let result = parse_format("%");
        assert_eq!(result.is_empty(), true);
    }

    #[test]
    fn parse_one_escaped_percent_i() {
        let result = parse_format("\\%");
        assert_eq!(result.is_empty(), true);
    }

    #[test]
    fn parse_one_escaped_percent_ii() {
        let result = parse_format("\\%%");
        assert_eq!(result.is_empty(), true);
    }

    #[test]
    fn parse_one_escaped_and_one_markup() {
        let result = parse_format("\\%%a");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, 'a');
        assert_eq!(result[0].1, 3);
    }

    #[test]
    fn parse_peaceful_markup() {
        let result = parse_format("%a");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, 'a');
        assert_eq!(result[0].1, 1);
    }

    #[test]
    fn parse_easy_markup() {
        let result = parse_format("\\%a%b");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, 'b');
        assert_eq!(result[0].1, 4);
    }

    #[test]
    fn parse_normal_markup() {
        let result = parse_format("\\%a%b%c");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, 'b');
        assert_eq!(result[0].1, 4);
        assert_eq!(result[1].0, 'c');
        assert_eq!(result[1].1, 6);
    }

    #[test]
    fn parse_hard_markup() {
        let result = parse_format("\\%a%b%c \\% %a\\% %");
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].0, 'b');
        assert_eq!(result[0].1, 4);
        assert_eq!(result[1].0, 'c');
        assert_eq!(result[1].1, 6);
        assert_eq!(result[2].0, 'a');
        assert_eq!(result[2].1, 12);
    }
}
