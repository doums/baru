// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::battery::Battery;
use crate::brightness::Brightness;
use crate::cpu::Cpu;
use crate::date_time::DateTime;
use crate::error::Error;
use crate::memory::Memory;
use crate::mic::Mic;
use crate::sound::Sound;
use crate::temperature::Temperature;
use crate::wired::Wired;
use crate::wireless::Wireless;
use crate::Config;
use crate::ModuleMsg;
use crate::Pulse;
use std::convert::TryFrom;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

pub type RunPtr = fn(char, Config, Arc<Mutex<Pulse>>, Sender<ModuleMsg>) -> Result<(), Error>;

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
    DateTime(DateTime<'a>),
    Memory(Memory<'a>),
    Mic(Mic<'a>),
    Wired(Wired<'a>),
    Sound(Sound<'a>),
    Temperature(Temperature<'a>),
    Wireless(Wireless<'a>),
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
            'i' => Ok(Module::Mic(Mic::with_config(config))),
            'm' => Ok(Module::Memory(Memory::with_config(config))),
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
            Module::DateTime(m) => m.name(),
            Module::Memory(m) => m.name(),
            Module::Mic(m) => m.name(),
            Module::Wired(m) => m.name(),
            Module::Sound(m) => m.name(),
            Module::Temperature(m) => m.name(),
            Module::Wireless(m) => m.name(),
        }
    }

    fn run_fn(&self) -> RunPtr {
        match self {
            Module::Battery(m) => m.run_fn(),
            Module::Brightness(m) => m.run_fn(),
            Module::Cpu(m) => m.run_fn(),
            Module::DateTime(m) => m.run_fn(),
            Module::Memory(m) => m.run_fn(),
            Module::Mic(m) => m.run_fn(),
            Module::Sound(m) => m.run_fn(),
            Module::Temperature(m) => m.run_fn(),
            Module::Wired(m) => m.run_fn(),
            Module::Wireless(m) => m.run_fn(),
        }
    }

    fn placeholder(&self) -> &str {
        match self {
            Module::Battery(m) => m.placeholder(),
            Module::Brightness(m) => m.placeholder(),
            Module::Cpu(m) => m.placeholder(),
            Module::DateTime(m) => m.placeholder(),
            Module::Memory(m) => m.placeholder(),
            Module::Wired(m) => m.placeholder(),
            Module::Mic(m) => m.placeholder(),
            Module::Sound(m) => m.placeholder(),
            Module::Temperature(m) => m.placeholder(),
            Module::Wireless(m) => m.placeholder(),
        }
    }

    fn format(&self) -> &str {
        match self {
            Module::Battery(m) => m.format(),
            Module::Brightness(m) => m.format(),
            Module::Cpu(m) => m.format(),
            Module::DateTime(m) => m.format(),
            Module::Memory(m) => m.format(),
            Module::Wired(m) => m.format(),
            Module::Mic(m) => m.format(),
            Module::Sound(m) => m.format(),
            Module::Temperature(m) => m.format(),
            Module::Wireless(m) => m.format(),
        }
    }
}

#[derive(Debug)]
pub struct ModuleData<'a> {
    pub key: char,
    pub module: Module<'a>,
    data: Option<String>,
}

impl<'a> ModuleData<'a> {
    pub fn new(key: char, config: &'a Config) -> Result<Self, Error> {
        Ok(ModuleData {
            key,
            module: Module::try_from((key, config))?,
            data: None,
        })
    }

    pub fn new_data(&mut self, value: Option<&str>, label: Option<&str>) {
        let mut module_format = self.module.format().to_string();
        module_format = match value {
            Some(v) => module_format.replace("%v", &v),
            None => module_format.replace("%v", ""),
        };
        module_format = match label {
            Some(l) => module_format.replace("%l", &l),
            None => module_format.replace("%l", ""),
        };
        self.data = Some(module_format);
    }

    pub fn output(&self) -> &str {
        if let Some(data) = &self.data {
            data
        } else {
            self.module.placeholder()
        }
    }
}
