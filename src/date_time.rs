// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::module::{Bar, RunPtr};
use crate::Pulse;
use crate::{Config as MainConfig, ModuleMsg};
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const PLACEHOLDER: &str = "-";
const DATE_FORMAT: &str = "%a. %-e %B %Y, %-kh%M";
const TICK_RATE: Duration = Duration::from_millis(500);
const FORMAT: &str = "%v";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    date_format: Option<String>,
    tick: Option<u32>,
    placeholder: Option<String>,
    label: Option<String>,
    format: Option<String>,
}

#[derive(Debug)]
pub struct InternalConfig<'a> {
    date_format: &'a str,
    tick: Duration,
    label: Option<&'a str>,
}

impl<'a> From<&'a MainConfig> for InternalConfig<'a> {
    fn from(config: &'a MainConfig) -> Self {
        let mut tick = TICK_RATE;
        let mut date_format = DATE_FORMAT;
        let mut label = None;
        if let Some(c) = &config.date_time {
            if let Some(d) = &c.date_format {
                date_format = d;
            }
            if let Some(t) = c.tick {
                tick = Duration::from_millis(t as u64)
            }
            label = c.label.as_deref();
        }
        InternalConfig {
            date_format,
            tick,
            label,
        }
    }
}

#[derive(Debug)]
pub struct DateTime<'a> {
    placeholder: &'a str,
    config: &'a MainConfig,
    format: &'a str,
}

impl<'a> DateTime<'a> {
    pub fn with_config(config: &'a MainConfig) -> Self {
        let mut placeholder = PLACEHOLDER;
        let mut format = FORMAT;
        if let Some(c) = &config.date_time {
            if let Some(p) = &c.placeholder {
                placeholder = p
            }
            if let Some(v) = &c.format {
                format = v;
            }
        }
        DateTime {
            placeholder,
            config,
            format,
        }
    }
}

impl<'a> Bar for DateTime<'a> {
    fn name(&self) -> &str {
        "date_time"
    }

    fn run_fn(&self) -> RunPtr {
        run
    }

    fn placeholder(&self) -> &str {
        self.placeholder
    }

    fn format(&self) -> &str {
        self.format
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
        tx.send(ModuleMsg(
            key,
            Some(Local::now().format(config.date_format).to_string()),
            config.label.map(|v| v.to_string()),
        ))?;
        thread::sleep(config.tick);
    }
}
