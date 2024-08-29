// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::modules::battery::Battery;
use crate::modules::brightness::Brightness;
use crate::modules::cpu::Cpu;
use crate::modules::cpu_freq::CpuFreq;
use crate::modules::date_time::DateTime;
use crate::modules::memory::Memory;
use crate::modules::mic::Mic;
use crate::modules::sound::Sound;
use crate::modules::temperature::Temperature;
use crate::modules::weather::Weather;
use crate::modules::wired::Wired;
use crate::modules::wireless::Wireless;
use crate::Config;
use crate::ModuleMsg;

use anyhow::{anyhow, Result};
use std::convert::TryFrom;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;
use tracing::{error, info, instrument};

const MODULE_FAILED_ICON: &str = "✗";

pub type RunPtr = fn(&AtomicBool, char, Config, Sender<ModuleMsg>) -> Result<(), Error>;

pub trait Bar {
    fn name(&self) -> &str;
    fn run_fn(&self) -> RunPtr;
    fn placeholder(&self) -> &str;
    fn format(&self) -> &str;
}

#[derive(Debug)]
pub enum Module<'a> {
    Battery(Battery<'a>),
    Brightness(Brightness<'a>),
    Cpu(Cpu<'a>),
    CpuFreq(CpuFreq<'a>),
    DateTime(DateTime<'a>),
    Memory(Memory<'a>),
    Mic(Mic<'a>),
    Wired(Wired<'a>),
    Sound(Sound<'a>),
    Temperature(Temperature<'a>),
    Wireless(Wireless<'a>),
    Weather(Weather<'a>),
}

impl<'a> TryFrom<(char, &'a Config)> for Module<'a> {
    type Error = Error;

    fn try_from((key, config): (char, &'a Config)) -> Result<Self, Self::Error> {
        match key {
            'a' => Ok(Module::Battery(Battery::with_config(config))),
            'b' => Ok(Module::Brightness(Brightness::with_config(config))),
            'c' => Ok(Module::Cpu(Cpu::with_config(config))),
            'd' => Ok(Module::DateTime(DateTime::with_config(config))),
            'e' => Ok(Module::Wired(Wired::with_config(config))),
            'f' => Ok(Module::CpuFreq(CpuFreq::with_config(config))),
            'i' => Ok(Module::Mic(Mic::with_config(config))),
            'm' => Ok(Module::Memory(Memory::with_config(config))),
            'r' => Ok(Module::Weather(Weather::with_config(config))),
            's' => Ok(Module::Sound(Sound::with_config(config))),
            't' => Ok(Module::Temperature(Temperature::with_config(config))),
            'w' => Ok(Module::Wireless(Wireless::with_config(config))),
            _ => Err(Error::new(format!("unknown markup \"{}\"", key))),
        }
    }
}

impl<'a> Bar for Module<'a> {
    fn name(&self) -> &str {
        match self {
            Module::Battery(m) => m.name(),
            Module::Brightness(m) => m.name(),
            Module::Cpu(m) => m.name(),
            Module::CpuFreq(m) => m.name(),
            Module::DateTime(m) => m.name(),
            Module::Memory(m) => m.name(),
            Module::Mic(m) => m.name(),
            Module::Wired(m) => m.name(),
            Module::Sound(m) => m.name(),
            Module::Temperature(m) => m.name(),
            Module::Weather(m) => m.name(),
            Module::Wireless(m) => m.name(),
        }
    }

    fn run_fn(&self) -> RunPtr {
        match self {
            Module::Battery(m) => m.run_fn(),
            Module::Brightness(m) => m.run_fn(),
            Module::Cpu(m) => m.run_fn(),
            Module::CpuFreq(m) => m.run_fn(),
            Module::DateTime(m) => m.run_fn(),
            Module::Memory(m) => m.run_fn(),
            Module::Mic(m) => m.run_fn(),
            Module::Sound(m) => m.run_fn(),
            Module::Temperature(m) => m.run_fn(),
            Module::Weather(m) => m.run_fn(),
            Module::Wired(m) => m.run_fn(),
            Module::Wireless(m) => m.run_fn(),
        }
    }

    fn placeholder(&self) -> &str {
        match self {
            Module::Battery(m) => m.placeholder(),
            Module::Brightness(m) => m.placeholder(),
            Module::Cpu(m) => m.placeholder(),
            Module::CpuFreq(m) => m.placeholder(),
            Module::DateTime(m) => m.placeholder(),
            Module::Memory(m) => m.placeholder(),
            Module::Wired(m) => m.placeholder(),
            Module::Mic(m) => m.placeholder(),
            Module::Sound(m) => m.placeholder(),
            Module::Temperature(m) => m.placeholder(),
            Module::Weather(m) => m.placeholder(),
            Module::Wireless(m) => m.placeholder(),
        }
    }

    fn format(&self) -> &str {
        match self {
            Module::Battery(m) => m.format(),
            Module::Brightness(m) => m.format(),
            Module::Cpu(m) => m.format(),
            Module::CpuFreq(m) => m.format(),
            Module::DateTime(m) => m.format(),
            Module::Memory(m) => m.format(),
            Module::Wired(m) => m.format(),
            Module::Mic(m) => m.format(),
            Module::Sound(m) => m.format(),
            Module::Temperature(m) => m.format(),
            Module::Weather(m) => m.format(),
            Module::Wireless(m) => m.format(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ModuleState {
    NotStarted,
    Running,
    /// Module's `run` function returned without errors
    Finished,
    /// Module's `run` function returned an error or panicked
    Failed,
}

#[derive(Debug)]
pub struct ModuleData<'a> {
    pub key: char,
    pub module: Module<'a>,
    data: Option<String>,
    state: ModuleState,
    handle: Option<JoinHandle<Result<(), Error>>>,
    failed_placeholder: String,
}

impl<'a> ModuleData<'a> {
    pub fn new(key: char, config: &'a Config) -> Result<Self> {
        Ok(ModuleData {
            key,
            module: Module::try_from((key, config))?,
            data: None,
            state: ModuleState::NotStarted,
            handle: None,
            failed_placeholder: config
                .failed_icon
                .as_ref()
                .map(|icon| format!("{}:{}", &key, icon))
                .unwrap_or_else(|| format!("{}:{}", &key, MODULE_FAILED_ICON)),
        })
    }

    pub fn new_data(&mut self, value: Option<&str>, label: Option<&str>) {
        let mut module_format = self.module.format().to_string();
        module_format = match value {
            Some(v) => module_format.replace("%v", v),
            None => module_format.replace("%v", ""),
        };
        module_format = match label {
            Some(l) => module_format.replace("%l", l),
            None => module_format.replace("%l", ""),
        };
        self.data = Some(module_format);
    }

    pub fn output(&self) -> &str {
        if matches!(self.state, ModuleState::Failed) {
            return &self.failed_placeholder;
        }

        if let Some(data) = &self.data {
            data
        } else {
            self.module.placeholder()
        }
    }

    pub fn start(&mut self, handle: JoinHandle<Result<(), Error>>) {
        self.handle = Some(handle);
        self.state = ModuleState::Running;
    }

    #[instrument(skip_all)]
    pub fn update_state(&mut self) -> Result<()> {
        let Some(handle) = &self.handle else {
            return Ok(());
        };
        if !handle.is_finished() {
            return Ok(());
        }

        // module thread has finished for some reason, join it
        // and update the state accordingly
        self.state = match self
            .handle
            .take()
            .ok_or(anyhow!("failed to unwrap handle"))?
            .join()
        {
            Ok(Ok(_)) => {
                info!("[{}] module finished", self.module.name());
                ModuleState::Finished
            }
            Ok(Err(e)) => {
                error!("[{}] module failed: {}", self.module.name(), e);
                ModuleState::Failed
            }
            Err(_) => {
                error!("[{}] module panicked", self.module.name());
                ModuleState::Failed
            }
        };
        Ok(())
    }

    #[instrument(skip_all)]
    pub fn _terminate(&mut self) {
        if let Some(handle) = self.handle.take() {
            match handle.join() {
                Ok(Ok(_)) => info!("[{}] module terminated", self.module.name()),
                Ok(Err(e)) => error!("[{}] module failed: {}", self.module.name(), e),
                Err(_) => error!("[{}] module panicked", self.module.name()),
            };
        }
    }
}
