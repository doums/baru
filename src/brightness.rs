// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::{read_and_parse, BarModule, Config as MainConfig};
use serde::{Deserialize, Serialize};

const SYS_PATH: &'static str =
    "/sys/devices/pci0000:00/0000:00:02.0/drm/card0/card0-eDP-1/intel_backlight";

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    sys_path: Option<String>,
}

#[derive(Debug)]
pub struct Brightness<'a> {
    sys_path: &'a str,
    config: &'a MainConfig,
}

impl<'a> Brightness<'a> {
    pub fn with_config(config: &'a MainConfig) -> Self {
        let mut sys_path = SYS_PATH;
        if let Some(c) = &config.brightness {
            if let Some(v) = &c.sys_path {
                sys_path = &v;
            }
        }
        Brightness { sys_path, config }
    }
}

impl<'a> BarModule for Brightness<'a> {
    fn refresh(&mut self) -> Result<String, Error> {
        let brightness = read_and_parse(&format!("{}/actual_brightness", self.sys_path))?;
        let max_brightness = read_and_parse(&format!("{}/max_brightness", self.sys_path))?;
        let percentage = 100 * brightness / max_brightness;
        Ok(format!(
            "{:3}% {}󰃟{}",
            percentage, self.config.icon_font, self.config.default_font
        ))
    }
}
