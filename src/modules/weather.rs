// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::http::HTTP_CLIENT;
use crate::module::{Bar, RunPtr};
use crate::{Config as MainConfig, ModuleMsg};
use regex::{Captures, Regex};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};
use tracing::{debug, error, instrument, trace, warn};

const PLACEHOLDER: &str = "-";
const TICK_RATE: Duration = Duration::from_secs(300);
const WTTR_URL: &str = "https://wttr.in";
const WTTR_FORMAT: &str = "%C+%t";
const LABEL: &str = "wtr";
const FORMAT: &str = "%l:%v";

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
enum Unit {
    /// SI units
    #[default]
    Metric,
    /// SI units with wind speed in m/s
    MetricMs,
    /// for US
    Uscs,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    location: Option<String>,
    wttr_format: Option<String>,
    unit: Option<Unit>,
    lang: Option<String>,
    /// remove leading '+' and trailing unit from temperature output
    compact_temp: Option<bool>,
    /// Update interval in minutes
    tick: Option<u32>,
    placeholder: Option<String>,
    label: Option<String>,
    format: Option<String>,
}

#[derive(Debug)]
pub struct InternalConfig<'a> {
    location: Option<&'a str>,
    wttr_format: &'a str,
    unit: Unit,
    compact_temp: bool,
    lang: Option<&'a str>,
    tick: Duration,
    label: &'a str,
}

impl<'a> Default for InternalConfig<'a> {
    fn default() -> Self {
        InternalConfig {
            location: None,
            wttr_format: WTTR_FORMAT,
            unit: Unit::default(),
            compact_temp: false,
            lang: None,
            tick: TICK_RATE,
            label: LABEL,
        }
    }
}

impl<'a> TryFrom<&'a MainConfig> for InternalConfig<'a> {
    type Error = Error;

    fn try_from(config: &'a MainConfig) -> Result<Self, Self::Error> {
        let internal_cfg = config
            .weather
            .as_ref()
            .map(|c| InternalConfig {
                location: c.location.as_deref(),
                wttr_format: c.wttr_format.as_deref().unwrap_or(WTTR_FORMAT),
                unit: c.unit.to_owned().unwrap_or_default(),
                compact_temp: c.compact_temp.unwrap_or(false),
                lang: c.lang.as_deref(),
                tick: c
                    .tick
                    .map_or(TICK_RATE, |t| Duration::from_secs((t * 60) as u64)),
                label: c.label.as_deref().unwrap_or(LABEL),
            })
            .unwrap_or_default();

        Ok(internal_cfg)
    }
}

#[derive(Debug)]
pub struct Weather<'a> {
    placeholder: &'a str,
    format: &'a str,
}

impl<'a> Weather<'a> {
    pub fn with_config(config: &'a MainConfig) -> Self {
        Weather {
            placeholder: config
                .weather
                .as_ref()
                .and_then(|c| c.placeholder.as_deref())
                .unwrap_or(PLACEHOLDER),
            format: config
                .weather
                .as_ref()
                .and_then(|c| c.format.as_deref())
                .unwrap_or(FORMAT),
        }
    }
}

impl<'a> Bar for Weather<'a> {
    fn name(&self) -> &str {
        "weather"
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

fn shrink_temp(re: &Regex, text: &str) -> String {
    re.replace_all(text, |caps: &Captures| {
        let mut temp = caps.name("temp").map_or("", |m| m.as_str()).to_string();
        let sign = caps.name("sign").map_or("", |m| m.as_str());
        if sign == "-" {
            temp.insert(0, '-')
        }
        temp
    })
    .to_string()
}

#[instrument(skip_all)]
pub fn run(key: char, main_config: MainConfig, tx: Sender<ModuleMsg>) -> Result<(), Error> {
    let re = Regex::new(r"(?<sign>[+-]?)(?<temp>\d+°)(?<unit>[CFcf])").unwrap();
    let config = InternalConfig::try_from(&main_config)?;
    debug!("{:#?}", config);
    let mut iteration_start: Instant;
    let mut iteration_end: Duration;
    let mut url = format!(
        "{WTTR_URL}/{}?format={}",
        config.location.unwrap_or(""),
        config.wttr_format
    );
    match config.unit {
        Unit::Metric => url.push_str("&m"),
        Unit::MetricMs => url.push_str("&M"),
        Unit::Uscs => url.push_str("&u"),
    }
    if let Some(l) = config.lang {
        url.push_str(&format!("&lang={}", l));
    }
    debug!("wttr URL: {}", url);
    loop {
        iteration_start = Instant::now();
        let response = HTTP_CLIENT
            .get(&url)
            .send()
            .inspect_err(|e| error!("request failed, {}", e))
            .ok();
        if let Some(res) = response {
            let output = res
                .text()
                .inspect_err(|e| error!("failed to parse response body, {}", e))
                .inspect(|text| trace!("response body: {}", text))
                .ok()
                .map(|text| {
                    if config.compact_temp {
                        shrink_temp(&re, &text)
                    } else {
                        text
                    }
                });
            if let Some(text) = output {
                tx.send(ModuleMsg(key, Some(text), Some(config.label.to_owned())))?;
            }
        }
        iteration_end = iteration_start.elapsed();
        if iteration_end < config.tick {
            thread::sleep(config.tick - iteration_end);
        }
    }
}
