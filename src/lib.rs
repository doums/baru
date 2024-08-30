// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

pub mod cli;
mod error;
mod http;
mod module;
mod modules;
mod netlink;
mod pulse;
pub mod signal;
pub mod trace;
pub mod util;

use anyhow::{anyhow, Result};
use error::Error;
use module::{Bar, ModuleData};
use modules::battery::Config as BatteryConfig;
use modules::brightness::Config as BrightnessConfig;
use modules::cpu_freq::Config as CpuFreqConfig;
use modules::cpu_usage::Config as CpuUsageConfig;
use modules::date_time::Config as DateTimeConfig;
use modules::memory::Config as MemoryConfig;
use modules::mic::Config as MicConfig;
use modules::sound::Config as SoundConfig;
use modules::temperature::Config as TemperatureConfig;
use modules::weather::Config as WeatherConfig;
use modules::wired::Config as WiredConfig;
use modules::wireless::Config as WirelessConfig;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::thread::JoinHandle;
use tracing::{error, info, instrument};

// Global application state, used to terminate the main-loop and all modules
pub static RUN: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(true));

#[derive(Debug)]
/// Message sent by modules.
/// `0`: module key,
/// `1`: value,
/// `2`: label
pub struct ModuleMsg(char, Option<String>, Option<String>);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    format: String,
    pub tick: Option<u32>,
    failed_icon: Option<String>,
    pulse_tick: Option<u32>,
    battery: Option<BatteryConfig>,
    brightness: Option<BrightnessConfig>,
    cpu_usage: Option<CpuUsageConfig>,
    cpu_freq: Option<CpuFreqConfig>,
    date_time: Option<DateTimeConfig>,
    memory: Option<MemoryConfig>,
    mic: Option<MicConfig>,
    sound: Option<SoundConfig>,
    temperature: Option<TemperatureConfig>,
    weather: Option<WeatherConfig>,
    wired: Option<WiredConfig>,
    wireless: Option<WirelessConfig>,
}

pub struct Baru<'a> {
    config: &'a Config,
    modules: Vec<ModuleData<'a>>,
    format: &'a str,
    markup_matches: Vec<MarkupMatch>,
    channel: (Sender<ModuleMsg>, Receiver<ModuleMsg>),
    pulse: Option<JoinHandle<Result<(), Error>>>,
}

#[derive(Debug)]
struct MarkupMatch(char, usize);

impl<'a> Baru<'a> {
    #[instrument(skip_all)]
    pub fn with_config(config: &'a Config) -> Result<Self> {
        let mut modules = vec![];
        let markup_matches = parse_format(&config.format);
        for markup in &markup_matches {
            modules.push(ModuleData::new(markup.0, config)?);
        }
        Ok(Baru {
            config,
            channel: mpsc::channel(),
            modules,
            format: &config.format,
            markup_matches,
            pulse: None,
        })
    }

    #[instrument(skip_all)]
    pub fn start(&mut self) -> Result<()> {
        // check if any module needs pulse, i.e. sound or mic modules
        let need_pulse = self.modules.iter().any(|m| m.key == 's' || m.key == 'i');
        if need_pulse {
            self.pulse = Some(pulse::init(self.config)?);
        }
        for data in &mut self.modules {
            let builder = thread::Builder::new().name(format!("mod_{}", data.module.name()));
            let cloned_m_conf = self.config.clone();
            let tx1 = mpsc::Sender::clone(&self.channel.0);
            let run = data.module.run_fn();
            let key = data.key;
            let c_name = data.module.name().to_string();
            let handle = builder.spawn(move || -> Result<(), Error> {
                run(&RUN, key, cloned_m_conf, tx1)
                    .inspect_err(|e| error!("[{}] module failed: {}", c_name, e))?;
                info!("[{}] module stopped", c_name);
                Ok(())
            })?;
            data.start(handle);
            info!("[{}] module started", data.module.name());
        }
        Ok(())
    }

    #[instrument(skip(self))]
    fn module_output(&self, key: char) -> Result<&str> {
        let module = self
            .modules
            .iter()
            .find(|data| data.key == key)
            .ok_or(anyhow!("module for key \"{}\" not found", key))?;
        Ok(module.output())
    }

    #[instrument(skip(self))]
    pub fn update(&mut self) -> Result<()> {
        let messages: Vec<ModuleMsg> = self.channel.1.try_iter().collect();
        for module in &mut self.modules {
            module.update_state().ok();
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

    #[instrument(skip(self))]
    pub fn modules(&self) -> Vec<&str> {
        self.modules.iter().map(|m| m.module.name()).collect()
    }

    #[instrument(skip_all)]
    pub fn cleanup(&mut self) {
        if let Some(pulse) = self.pulse.take() {
            match pulse.join() {
                Ok(Ok(_)) => info!("pulse module terminated"),
                Ok(Err(e)) => error!("pulse module failed: {}", e),
                Err(_) => error!("pulse module panicked"),
            };
        }
    }
}

#[instrument]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_format() {
        let result = parse_format("");
        assert!(result.is_empty());
    }

    #[test]
    fn parse_one_char() {
        let result = parse_format("a");
        assert!(result.is_empty());
    }

    #[test]
    fn parse_one_percent() {
        let result = parse_format("%");
        assert!(result.is_empty());
    }

    #[test]
    fn parse_one_escaped_percent_i() {
        let result = parse_format("\\%");
        assert!(result.is_empty());
    }

    #[test]
    fn parse_one_escaped_percent_ii() {
        let result = parse_format("\\%%");
        assert!(result.is_empty());
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
