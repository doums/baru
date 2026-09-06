// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::module::{Bar, RunPtr};
use crate::{Config as MainConfig, ModuleMsg};
use serde::Deserialize;
use std::convert::TryFrom;
use std::fs;
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
const NOT_CHARGING_LABEL: &str = "'ba";
const UNKNOWN_LABEL: &str = ".ba";
const LOW_LEVEL: u32 = 10;
const TICK_RATE: Duration = Duration::from_millis(500);
const FORMAT: &str = "%l:%v";

#[derive(Debug, Deserialize, Clone)]
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
    not_charging_label: Option<String>,
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
    not_charging_label: &'a str,
    unknown_label: &'a str,
}

#[derive(Debug)]
enum BatteryStatus {
    Full,
    Discharging,
    Charging,
    NotCharging,
    Unknown,
}

impl From<&str> for BatteryStatus {
    fn from(value: &str) -> Self {
        match value {
            "Full" => Self::Full,
            "Discharging" => Self::Discharging,
            "Charging" => Self::Charging,
            "Not charging" => Self::NotCharging,
            _ => Self::Unknown,
        }
    }
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
        let mut not_charging_label = NOT_CHARGING_LABEL;
        let mut unknown_label = UNKNOWN_LABEL;
        if let Some(c) = &config.battery {
            if let Some(n) = &c.name {
                name = n;
            }
            if let Some(v) = &c.low_level {
                low_level = *v;
            }
            if let Some(b) = c.full_design
                && b
            {
                full_design = true;
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
            if let Some(v) = &c.not_charging_label {
                not_charging_label = v;
            }
            if let Some(v) = &c.unknown_label {
                unknown_label = v;
            }
        }
        let full_attr = match full_design {
            true => FULL_DESIGN_ATTRIBUTE,
            false => FULL_ATTRIBUTE,
        };
        let uevent = format!("{}{}/{}", SYS_PATH, name, UEVENT);
        let attribute_prefix = find_attribute_prefix(&uevent)?;
        Ok(InternalConfig {
            low_level,
            tick,
            uevent,
            now_attribute: format!("{POWER_SUPPLY}_{attribute_prefix}_{NOW_ATTRIBUTE}"),
            full_attribute: format!("{POWER_SUPPLY}_{attribute_prefix}_{full_attr}"),
            full_label,
            charging_label,
            discharging_label,
            low_label,
            not_charging_label,
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
        let label = match status {
            BatteryStatus::Full => config.full_label,
            BatteryStatus::Discharging => {
                if battery_level <= config.low_level {
                    config.low_label
                } else {
                    config.discharging_label
                }
            }
            BatteryStatus::Charging => config.charging_label,
            BatteryStatus::NotCharging => config.not_charging_label,
            BatteryStatus::Unknown => config.unknown_label,
        };
        tx.send(ModuleMsg(
            key,
            Some(format!("{battery_level:3}%")),
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
) -> Result<(i32, i32, BatteryStatus), Error> {
    let mut now = None;
    let mut full = None;
    let mut status = None;
    for line in fs::read_to_string(uevent)
        .map_err(|e| Error::new(format!("failed to read uevent file {uevent}: {e}")))?
        .lines()
    {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if now.is_none() && key == now_attribute {
            now = Some(
                value
                    .parse()
                    .map_err(|e| Error::new(format!("failed to parse value for {key}: {e}")))?,
            )
        }
        if full.is_none() && key == full_attribute {
            full = Some(
                value
                    .parse()
                    .map_err(|e| Error::new(format!("failed to parse value for {key}: {e}")))?,
            )
        }
        if status.is_none() && key == STATUS_ATTRIBUTE {
            status = Some(BatteryStatus::from(value));
        }
        if now.is_some() && full.is_some() && status.is_some() {
            break;
        }
    }
    if now.is_none() {
        return Err(Error::new(format!(
            "attribute '{}' not found in {}",
            now_attribute, uevent
        )));
    }
    if full.is_none() {
        return Err(Error::new(format!(
            "attribute '{}' not found in {}",
            full_attribute, uevent
        )));
    }
    if status.is_none() {
        return Err(Error::new(format!(
            "attribute '{}' not found in {}",
            STATUS_ATTRIBUTE, uevent
        )));
    }
    Ok((now.unwrap(), full.unwrap(), status.unwrap()))
}

fn find_attribute_prefix<'e>(path: &str) -> Result<&'e str, Error> {
    let content = fs::read_to_string(path)?;
    let mut unit = None;
    if content.contains(&format!(
        "{POWER_SUPPLY}_{ENERGY_PREFIX}_{FULL_DESIGN_ATTRIBUTE}="
    )) && content.contains(&format!("{POWER_SUPPLY}_{ENERGY_PREFIX}_{FULL_ATTRIBUTE}="))
        && content.contains(&format!("{POWER_SUPPLY}_{ENERGY_PREFIX}_{NOW_ATTRIBUTE}="))
    {
        unit = Some(ENERGY_PREFIX);
    } else if content.contains(&format!(
        "{POWER_SUPPLY}_{CHARGE_PREFIX}_{FULL_DESIGN_ATTRIBUTE}="
    )) && content.contains(&format!("{POWER_SUPPLY}_{CHARGE_PREFIX}_{FULL_ATTRIBUTE}="))
        && content.contains(&format!("{POWER_SUPPLY}_{CHARGE_PREFIX}_{NOW_ATTRIBUTE}="))
    {
        unit = Some(CHARGE_PREFIX);
    }
    unit.ok_or_else(|| Error::new(format!("unable to find the required attributes in {path}")))
}
