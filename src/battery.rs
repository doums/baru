// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::module::{BaruMod, RunPtr};
use crate::pulse::Pulse;
use crate::{Config as MainConfig, ModuleMsg};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::fs::{self, File};
use std::io::{self, prelude::*, BufReader};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const PLACEHOLDER: &str = "-";
const SYS_PATH: &str = "/sys/class/power_supply/";
const BATTERY_NAME: &str = "BAT0";
const UEVENT: &str = "uevent";
const POWER_SUPPLY: &str = "POWER_SUPPLY";
const CHARGE_PREFIX: &str = "CHARGE";
const ENERGY_PREFIX: &str = "ENERGY";
const FULL_ATTRIBUTE: &str = "FULL";
const FULL_DESIGN_ATTRIBUTE: &str = "FULL_DESIGN";
const NOW_ATTRIBUTE: &str = "NOW";
const STATUS_ATTRIBUTE: &str = "POWER_SUPPLY_STATUS";
const FULL_LABEL: &str = "*ba";
const CHARGING_LABEL: &str = "^ba";
const DISCHARGING_LABEL: &str = "bat";
const LOW_LABEL: &str = "!ba";
const UNKNOWN_LABEL: &str = ".ba";
const LOW_LEVEL: u32 = 10;
const TICK_RATE: Duration = Duration::from_millis(500);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    name: Option<String>,
    low_level: Option<u32>,
    full_design: Option<bool>,
    tick: Option<u32>,
    placeholder: Option<String>,
    full_text: Option<String>,
    charging_text: Option<String>,
    discharging_text: Option<String>,
    low_text: Option<String>,
    unknown_text: Option<String>,
}

#[derive(Debug)]
pub struct InternalConfig<'a> {
    name: &'a str,
    low_level: u32,
    full_design: bool,
    tick: Duration,
    uevent: String,
    now_attribute: String,
    full_attribute: String,
    full_text: &'a str,
    charging_text: &'a str,
    discharging_text: &'a str,
    low_text: &'a str,
    unknown_text: &'a str,
}

impl<'a> TryFrom<&'a MainConfig> for InternalConfig<'a> {
    type Error = Error;

    fn try_from(config: &'a MainConfig) -> Result<Self, Self::Error> {
        let mut low_level = LOW_LEVEL;
        let mut name = BATTERY_NAME;
        let mut full_design = false;
        let mut tick = TICK_RATE;
        let mut full_text = FULL_LABEL;
        let mut charging_text = CHARGING_LABEL;
        let mut discharging_text = DISCHARGING_LABEL;
        let mut low_text = LOW_LABEL;
        let mut unknown_text = UNKNOWN_LABEL;
        if let Some(c) = &config.battery {
            if let Some(n) = &c.name {
                name = n;
            }
            if let Some(v) = &c.low_level {
                low_level = *v;
            }
            if let Some(b) = c.full_design {
                if b {
                    full_design = true;
                }
            }
            if let Some(t) = c.tick {
                tick = Duration::from_millis(t as u64)
            }
            if let Some(v) = &c.full_text {
                full_text = v;
            }
            if let Some(v) = &c.charging_text {
                charging_text = v;
            }
            if let Some(v) = &c.discharging_text {
                discharging_text = v;
            }
            if let Some(v) = &c.low_text {
                low_text = v;
            }
            if let Some(v) = &c.unknown_text {
                unknown_text = v;
            }
        }
        let full_attr = match full_design {
            true => FULL_DESIGN_ATTRIBUTE,
            false => FULL_ATTRIBUTE,
        };
        let uevent = format!("{}{}/{}", SYS_PATH, &name, UEVENT);
        let attribute_prefix = find_attribute_prefix(&uevent)?;
        Ok(InternalConfig {
            name,
            low_level,
            full_design,
            tick,
            uevent,
            now_attribute: format!("{}_{}_{}", POWER_SUPPLY, attribute_prefix, NOW_ATTRIBUTE),
            full_attribute: format!("{}_{}_{}", POWER_SUPPLY, attribute_prefix, full_attr),
            full_text,
            charging_text,
            discharging_text,
            low_text,
            unknown_text,
        })
    }
}

#[derive(Debug)]
pub struct Battery<'a> {
    placeholder: &'a str,
    config: &'a MainConfig,
}

impl<'a> Battery<'a> {
    pub fn with_config(config: &'a MainConfig) -> Self {
        let mut placeholder = PLACEHOLDER;
        if let Some(c) = &config.battery {
            if let Some(p) = &c.placeholder {
                placeholder = p
            }
        }
        Battery {
            placeholder,
            config,
        }
    }
}

impl<'a> BaruMod for Battery<'a> {
    fn run_fn(&self) -> RunPtr {
        run
    }

    fn placeholder(&self) -> &str {
        self.placeholder
    }

    fn name(&self) -> &str {
        "battery"
    }
}

pub fn run(
    key: char,
    main_config: MainConfig,
    _: Arc<Mutex<Pulse>>,
    tx: Sender<ModuleMsg>,
) -> Result<(), Error> {
    let config = InternalConfig::try_from(&main_config)?;
    loop {
        let (energy, capacity, status) = parse_attributes(
            &config.uevent,
            &config.now_attribute,
            &config.full_attribute,
        )?;
        let capacity = capacity as u64;
        let energy = energy as u64;
        let battery_level = u32::try_from(100_u64 * energy / capacity)?;
        let text = match status.as_str() {
            "Full" => config.full_text,
            "Discharging" => {
                if battery_level <= config.low_level {
                    config.low_text
                } else {
                    config.discharging_text
                }
            }
            "Charging" => config.charging_text,
            _ => config.unknown_text,
        };
        tx.send(ModuleMsg(key, format!("{:3}%{}", battery_level, text)))?;
        thread::sleep(config.tick);
    }
}

fn parse_attributes(
    uevent: &str,
    now_attribute: &str,
    full_attribute: &str,
) -> Result<(i32, i32, String), Error> {
    let file = File::open(&uevent)?;
    let f = BufReader::new(file);
    let mut now = None;
    let mut full = None;
    let mut status = None;
    for line in f.lines() {
        if now.is_none() {
            now = parse_attribute(&line, &now_attribute);
        }
        if full.is_none() {
            full = parse_attribute(&line, &full_attribute);
        }
        if status.is_none() {
            status = parse_status(&line);
        }
    }
    if now.is_none() || full.is_none() || status.is_none() {
        return Err(Error::new(format!(
            "unable to parse the required attributes in {}",
            uevent
        )));
    }
    Ok((now.unwrap(), full.unwrap(), status.unwrap()))
}

fn parse_attribute(line: &io::Result<String>, attribute: &str) -> Option<i32> {
    if let Ok(l) = line {
        if l.starts_with(&attribute) {
            let s = l.split('=').nth(1);
            if let Some(v) = s {
                return v.parse::<i32>().ok();
            }
        }
    }
    None
}

fn parse_status(line: &io::Result<String>) -> Option<String> {
    if let Ok(l) = line {
        if l.starts_with(&STATUS_ATTRIBUTE) {
            return l.split('=').nth(1).map(|s| s.to_string());
        }
    }
    None
}

fn find_attribute_prefix<'a, 'b>(path: &'a str) -> Result<&'b str, Error> {
    let content = fs::read_to_string(path)?;
    let mut unit = None;
    if content.contains(&format!(
        "{}_{}_{}=",
        POWER_SUPPLY, ENERGY_PREFIX, FULL_DESIGN_ATTRIBUTE
    )) && content.contains(&format!(
        "{}_{}_{}=",
        POWER_SUPPLY, ENERGY_PREFIX, FULL_ATTRIBUTE
    )) && content.contains(&format!(
        "{}_{}_{}=",
        POWER_SUPPLY, ENERGY_PREFIX, NOW_ATTRIBUTE
    )) {
        unit = Some(ENERGY_PREFIX);
    } else if content.contains(&format!(
        "{}_{}_{}=",
        POWER_SUPPLY, CHARGE_PREFIX, FULL_DESIGN_ATTRIBUTE
    )) && content.contains(&format!(
        "{}_{}_{}=",
        POWER_SUPPLY, CHARGE_PREFIX, FULL_ATTRIBUTE
    )) && content.contains(&format!(
        "{}_{}_{}=",
        POWER_SUPPLY, CHARGE_PREFIX, NOW_ATTRIBUTE
    )) {
        unit = Some(CHARGE_PREFIX);
    }
    unit.ok_or_else(|| {
        Error::new(format!(
            "unable to find the required attributes in {}",
            path
        ))
    })
}
