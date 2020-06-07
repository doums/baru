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
use crate::Pulse;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

pub trait BaruMod {
    fn run_fn(&self) -> fn(Config, Arc<Mutex<Pulse>>, Sender<String>) -> Result<(), Error>;
    fn placeholder(&self) -> &str;
}

enum Module<'a> {
    Battery(Battery<'a>),
    Brightness(Brightness<'a>),
    Cpu(Cpu<'a>),
    Mic(Mic<'a>),
    Wired(Wired<'a>),
    DateTime(DateTime<'a>),
    Memory(Memory<'a>),
    Sound(Sound<'a>),
    Temperature(Temperature<'a>),
    Wireless(Wireless<'a>),
}

impl<'a> BaruMod for Module<'a> {
    fn run_fn(&self) -> fn(Config, Arc<Mutex<Pulse>>, Sender<String>) -> Result<(), Error> {
        match self {
            Module::Battery(m) => m.run_fn(),
            Module::Brightness(m) => m.run_fn(),
            Module::Cpu(m) => m.run_fn(),
            Module::DateTime(m) => m.run_fn(),
            Module::Wired(m) => m.run_fn(),
            Module::Memory(m) => m.run_fn(),
            Module::Mic(m) => m.run_fn(),
            Module::Sound(m) => m.run_fn(),
            Module::Temperature(m) => m.run_fn(),
            Module::Wireless(m) => m.run_fn(),
        }
    }

    fn placeholder(&self) -> &str {
        match self {
            Module::Battery(m) => m.placeholder(),
            Module::Brightness(m) => m.placeholder(),
            Module::Cpu(m) => m.placeholder(),
            Module::DateTime(m) => m.placeholder(),
            Module::Wired(m) => m.placeholder(),
            Module::Memory(m) => m.placeholder(),
            Module::Mic(m) => m.placeholder(),
            Module::Sound(m) => m.placeholder(),
            Module::Temperature(m) => m.placeholder(),
            Module::Wireless(m) => m.placeholder(),
        }
    }
}

pub struct Wrapper<'a> {
    channel: (Sender<String>, Receiver<String>),
    prev_data: Option<String>,
    config: &'a Config,
    name: &'a str,
    pulse: &'a Arc<Mutex<Pulse>>,
    module: Module<'a>,
}

impl<'a> Wrapper<'a> {
    pub fn new(
        markup: char,
        config: &'a Config,
        pulse: &'a Arc<Mutex<Pulse>>,
    ) -> Result<Self, Error> {
        let (tx, rx) = mpsc::channel();
        match markup {
            'a' => Ok(Wrapper {
                channel: (tx, rx),
                config,
                prev_data: None,
                name: "battery",
                pulse,
                module: Module::Battery(Battery::with_config(config)),
            }),
            'b' => Ok(Wrapper {
                channel: (tx, rx),
                config,
                prev_data: None,
                name: "brightness",
                pulse,
                module: Module::Brightness(Brightness::with_config(config)),
            }),
            'c' => Ok(Wrapper {
                channel: (tx, rx),
                config,
                prev_data: None,
                name: "cpu",
                pulse,
                module: Module::Cpu(Cpu::with_config(config)),
            }),
            'd' => Ok(Wrapper {
                channel: (tx, rx),
                config,
                prev_data: None,
                name: "date_time",
                pulse,
                module: Module::DateTime(DateTime::with_config(config)),
            }),
            'e' => Ok(Wrapper {
                channel: (tx, rx),
                config,
                prev_data: None,
                name: "wired",
                pulse,
                module: Module::Wired(Wired::with_config(config)),
            }),
            'm' => Ok(Wrapper {
                channel: (tx, rx),
                config,
                prev_data: None,
                name: "memory",
                pulse,
                module: Module::Memory(Memory::with_config(config)),
            }),
            'i' => Ok(Wrapper {
                channel: (tx, rx),
                config,
                prev_data: None,
                name: "mic",
                pulse,
                module: Module::Mic(Mic::with_config(config)),
            }),
            's' => Ok(Wrapper {
                channel: (tx, rx),
                config,
                prev_data: None,
                name: "sound",
                pulse,
                module: Module::Sound(Sound::with_config(config)),
            }),
            't' => Ok(Wrapper {
                channel: (tx, rx),
                config,
                prev_data: None,
                name: "temperature",
                pulse,
                module: Module::Temperature(Temperature::with_config(config)),
            }),
            'w' => Ok(Wrapper {
                channel: (tx, rx),
                config,
                prev_data: None,
                name: "wireless",
                pulse,
                module: Module::Wireless(Wireless::with_config(config)),
            }),
            _ => Err(Error::new(format!("unknown markup \"{}\"", markup))),
        }
    }

    pub fn start(&mut self) -> Result<(), Error> {
        let builder = thread::Builder::new().name(format!("mod_{}", self.name));
        let cloned_m_conf = self.config.clone();
        let tx1 = mpsc::Sender::clone(&self.channel.0);
        let pulse = Arc::clone(self.pulse);
        let run = self.module.run_fn();
        builder.spawn(move || -> Result<(), Error> {
            run(cloned_m_conf, pulse, tx1)?;
            Ok(())
        })?;
        Ok(())
    }

    pub fn data(&self) -> Option<String> {
        self.channel.1.try_iter().last()
    }

    pub fn refresh(&mut self) -> Result<&str, Error> {
        if let Some(data) = self.data() {
            self.prev_data = Some(data);
        }
        if let Some(data) = &self.prev_data {
            Ok(data)
        } else {
            Ok(self.module.placeholder())
        }
    }
}
