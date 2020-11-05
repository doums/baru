// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::module::{BaruMod, RunPtr};
use crate::Pulse;
use crate::{Config as MainConfig, ModuleMsg};
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const PLACEHOLDER: &str = "-";
const FORMAT: &str = "%a. %-e %B %Y, %-kh%M";
const TICK_RATE: Duration = Duration::from_millis(500);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    format: Option<String>,
    tick: Option<u32>,
    placeholder: Option<String>,
    text: Option<String>,
}

#[derive(Debug)]
pub struct InternalConfig<'a> {
    format: &'a str,
    tick: Duration,
    text: Option<&'a str>,
}

impl<'a> From<&'a MainConfig> for InternalConfig<'a> {
    fn from(config: &'a MainConfig) -> Self {
        let mut tick = TICK_RATE;
        let mut format = FORMAT;
        let mut text = None;
        if let Some(c) = &config.date_time {
            if let Some(d) = &c.format {
                format = d;
            }
            if let Some(t) = c.tick {
                tick = Duration::from_millis(t as u64)
            }
            text = c.text.as_deref();
        }
        InternalConfig { format, tick, text }
    }
}

#[derive(Debug)]
pub struct DateTime<'a> {
    placeholder: &'a str,
    config: &'a MainConfig,
}

impl<'a> DateTime<'a> {
    pub fn with_config(config: &'a MainConfig) -> Self {
        let mut placeholder = PLACEHOLDER;
        if let Some(c) = &config.date_time {
            if let Some(p) = &c.placeholder {
                placeholder = p
            }
        }
        DateTime {
            placeholder,
            config,
        }
    }
}

impl<'a> BaruMod for DateTime<'a> {
    fn run_fn(&self) -> RunPtr {
        run
    }

    fn placeholder(&self) -> &str {
        self.placeholder
    }

    fn name(&self) -> &str {
        "date_time"
    }
}

pub fn run(
    key: char,
    main_config: MainConfig,
    _: Arc<Mutex<Pulse>>,
    tx: Sender<ModuleMsg>,
) -> Result<(), Error> {
    let config = InternalConfig::from(&main_config);
    loop {
        let now = Local::now();
        match config.text {
            Some(text) => tx.send(ModuleMsg(
                key,
                format!("{}{}", now.format(config.format), text),
            ))?,
            None => tx.send(ModuleMsg(key, now.format(config.format).to_string()))?,
        }
        thread::sleep(config.tick);
    }
}
