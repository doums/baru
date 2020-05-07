// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::{BarModule, Config as MainConfig};
use chrono::prelude::*;
use serde::{Deserialize, Serialize};

const FORMAT: &str = "%a. %-e %B %Y, %-kh%M";

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    format: Option<String>,
}

pub struct DateTime<'a>(&'a str);

impl<'a> DateTime<'a> {
    pub fn new(config: &'a MainConfig) -> Self {
        let mut format = FORMAT;
        if let Some(c) = &config.date_time {
            if let Some(d) = &c.format {
                format = d;
            }
        }
        DateTime(format)
    }
}

impl<'a> BarModule for DateTime<'a> {
    fn refresh(&mut self) -> Result<String, Error> {
        let now = Local::now();
        Ok(now.format(self.0).to_string())
    }
}
