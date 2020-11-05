// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::module::{BaruMod, RunPtr};
use crate::pulse::Pulse;
use crate::{Config as MainConfig, ModuleMsg};
use serde::{Deserialize, Serialize};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const PLACEHOLDER: &str = "-";
const TICK_RATE: Duration = Duration::from_millis(50);
const MUTE_LABEL: &str = ".so";
const LABEL: &str = "sou";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub index: Option<u32>,
    tick: Option<u32>,
    placeholder: Option<String>,
    text: Option<String>,
    mute_text: Option<String>,
}

#[derive(Debug)]
pub struct InternalConfig<'a> {
    tick: Duration,
    text: &'a str,
    mute_text: &'a str,
}

impl<'a> From<&'a MainConfig> for InternalConfig<'a> {
    fn from(config: &'a MainConfig) -> Self {
        let mut tick = TICK_RATE;
        let mut text = LABEL;
        let mut mute_text = MUTE_LABEL;
        if let Some(c) = &config.sound {
            if let Some(t) = c.tick {
                tick = Duration::from_millis(t as u64)
            }
            if let Some(v) = &c.text {
                text = v;
            }
            if let Some(v) = &c.mute_text {
                mute_text = v;
            }
        }
        InternalConfig {
            tick,
            text,
            mute_text,
        }
    }
}

#[derive(Debug)]
pub struct Sound<'a> {
    placeholder: &'a str,
    config: &'a MainConfig,
}

impl<'a> Sound<'a> {
    pub fn with_config(config: &'a MainConfig) -> Self {
        let mut placeholder = PLACEHOLDER;
        if let Some(c) = &config.sound {
            if let Some(p) = &c.placeholder {
                placeholder = p
            }
        }
        Sound {
            placeholder,
            config,
        }
    }
}

impl<'a> BaruMod for Sound<'a> {
    fn run_fn(&self) -> RunPtr {
        run
    }

    fn placeholder(&self) -> &str {
        self.placeholder
    }

    fn name(&self) -> &str {
        "sound"
    }
}

pub fn run(
    key: char,
    main_config: MainConfig,
    pulse: Arc<Mutex<Pulse>>,
    tx: Sender<ModuleMsg>,
) -> Result<(), Error> {
    let config = InternalConfig::from(&main_config);
    loop {
        if let Some(data) = pulse.lock().unwrap().output_data() {
            let text = match data.1 {
                true => config.mute_text,
                false => config.text,
            };
            tx.send(ModuleMsg(key, format!("{:3}%{}", data.0, text,)))?;
        }
        thread::sleep(config.tick);
    }
}
