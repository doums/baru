// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::module::{Bar, RunPtr};
use crate::{Config as MainConfig, ModuleMsg};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::fs::{self, File};
use std::io::{self, prelude::*, BufReader};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};
use tracing::{debug, instrument};

const PLACEHOLDER: &str = "-";
const SYS_PATH: &str = "/sys/class/power_supply/";
const BATTERY_NAME: &str = "BAT0";
const UEVENT: &str = "uevent";
const FULL_DESIGN: bool = false;
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
const FORMAT: &str = "%l:%v";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    name: Option<String>,
    low_level: Option<u32>,
    full_design: Option<bool>,
    tick: Option<u32>,
    placeholder: Option<String>,
    full_label: Option<String>,
    charging_label: Option<String>,
    discharging_label: Option<String>,
    low_label: Option<String>,
    unknown_label: Option<String>,
    format: Option<String>,
}

#[derive(Debug)]
pub struct InternalConfig<'a> {
    low_level: u32,
    tick: Duration,
    uevent: String,
    now_attribute: String,
    full_attribute: String,
    full_label: &'a str,
    charging_label: &'a str,
    discharging_label: &'a str,
    low_label: &'a str,
    unknown_label: &'a str,
}

impl<'a> TryFrom<&'a MainConfig> for InternalConfig<'a> {
    type Error = Error;

    fn try_from(config: &'a MainConfig) -> Result<Self, Self::Error> {
        let mut low_level = LOW_LEVEL;
        let mut name = BATTERY_NAME;
        let mut full_design = FULL_DESIGN;
        let mut tick = TICK_RATE;
        let mut full_label = FULL_LABEL;
        let mut charging_label = CHARGING_LABEL;
        let mut discharging_label = DISCHARGING_LABEL;
        let mut low_label = LOW_LABEL;
        let mut unknown_label = UNKNOWN_LABEL;
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
            if let Some(v) = &c.full_label {
                full_label = v;
            }
            if let Some(v) = &c.charging_label {
                charging_label = v;
            }
            if let Some(v) = &c.discharging_label {
                discharging_label = v;
            }
            if let Some(v) = &c.low_label {
                low_label = v;
            }
            if let Some(v) = &c.unknown_label {
                unknown_label = v;
            }
        }
        let full_attr = match full_design {
            true => FULL_DESIGN_ATTRIBUTE,
            false => FULL_ATTRIBUTE,
        };
        let uevent = format!("{}{}/{}", SYS_PATH, &name, UEVENT);
        let attribute_prefix = find_attribute_prefix(&uevent)?;
        Ok(InternalConfig {
            low_level,
            tick,
            uevent,
            now_attribute: format!("{}_{}_{}", POWER_SUPPLY, attribute_prefix, NOW_ATTRIBUTE),
            full_attribute: format!("{}_{}_{}", POWER_SUPPLY, attribute_prefix, full_attr),
            full_label,
            charging_label,
            discharging_label,
            low_label,
            unknown_label,
        })
    }
}

#[derive(Debug)]
pub struct Battery<'a> {
    placeholder: &'a str,
    format: &'a str,
}

impl<'a> Battery<'a> {
    pub fn with_config(config: &'a MainConfig) -> Self {
        let mut placeholder = PLACEHOLDER;
        let mut format = FORMAT;
        if let Some(c) = &config.battery {
            if let Some(p) = &c.placeholder {
                placeholder = p
            }
            if let Some(v) = &c.format {
                format = v;
            }
        }
        Battery {
            format,
            placeholder,
        }
    }
}

impl<'a> Bar for Battery<'a> {
    fn name(&self) -> &str {
        "battery"
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

#[instrument(skip_all)]
pub fn run(
    running: &AtomicBool,
    key: char,
    main_config: MainConfig,
    tx: Sender<ModuleMsg>,
) -> Result<(), Error> {
    let config = InternalConfig::try_from(&main_config)?;
    debug!("{:#?}", config);
    let mut iteration_start: Instant;
    let mut iteration_end: Duration;
    while running.load(Ordering::Relaxed) {
        iteration_start = Instant::now();
        let (energy, capacity, status) = parse_attributes(
            &config.uevent,
            &config.now_attribute,
            &config.full_attribute,
        )?;
        let capacity = capacity as u64;
        let energy = energy as u64;
        let battery_level = u32::try_from(100_u64 * energy / capacity)?;
        let label = match status.as_str() {
            "Full" => config.full_label,
            "Discharging" => {
                if battery_level <= config.low_level {
                    config.low_label
                } else {
                    config.discharging_label
                }
            }
            "Charging" => config.charging_label,
            _ => config.unknown_label,
        };
        tx.send(ModuleMsg(
            key,
            Some(format!("{:3}%", battery_level)),
            Some(label.to_string()),
        ))?;
        iteration_end = iteration_start.elapsed();
        if iteration_end < config.tick {
            thread::sleep(config.tick - iteration_end);
        }
    }
    Ok(())
}

fn parse_attributes(
    uevent: &str,
    now_attribute: &str,
    full_attribute: &str,
) -> Result<(i32, i32, String), Error> {
    let file = File::open(uevent)?;
    let f = BufReader::new(file);
    let mut now = None;
    let mut full = None;
    let mut status = None;
    for line in f.lines() {
        if now.is_none() {
            now = parse_attribute(&line, now_attribute);
        }
        if full.is_none() {
            full = parse_attribute(&line, full_attribute);
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
        if l.starts_with(attribute) {
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
        if l.starts_with(STATUS_ATTRIBUTE) {
            return l.split('=').nth(1).map(|s| s.to_string());
        }
    }
    None
}

fn find_attribute_prefix<'e>(path: &str) -> Result<&'e str, Error> {
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
