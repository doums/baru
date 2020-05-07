// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::pulse::{Pulse, PulseData};
use crate::{BarModule, Config as MainConfig};
use serde::{Deserialize, Serialize};

const HIGH_LEVEL: u32 = 100;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub index: Option<u32>,
    high_level: Option<u32>,
}

pub struct Mic<'a> {
    config: &'a MainConfig,
    pulse: &'a Pulse,
    prev_data: Option<PulseData>,
    high_level: u32,
}

impl<'a> Mic<'a> {
    pub fn with_config(config: &'a MainConfig, pulse: &'a Pulse) -> Self {
        let mut high_level = HIGH_LEVEL;
        if let Some(c) = &config.mic {
            if let Some(v) = c.high_level {
                high_level = v;
            }
        }
        Mic {
            config,
            pulse,
            prev_data: None,
            high_level,
        }
    }
}

impl<'a> BarModule for Mic<'a> {
    fn refresh(&mut self) -> Result<String, Error> {
        let data = self.pulse.input_data();
        if data.is_some() {
            self.prev_data = data;
        }
        let icon;
        let mut color = &self.config.default_color;
        if let Some(info) = self.prev_data {
            if info.1 {
                icon = "󰍭";
            } else {
                icon = "󰍬";
            }
            if info.0 > self.high_level as i32 {
                color = &self.config.red;
            }
            Ok(format!(
                "{:3}% {}{}{}{}{}",
                info.0,
                color,
                self.config.icon_font,
                icon,
                self.config.default_font,
                self.config.default_color
            ))
        } else {
            icon = "󰍮";
            Ok(format!(
                "     {}{}{}",
                self.config.icon_font, icon, self.config.default_font
            ))
        }
    }
}
