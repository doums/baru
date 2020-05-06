// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::BarModule;
use chrono::prelude::*;

#[derive(Debug)]
pub struct DateTime;

impl DateTime {
    pub fn new() -> Self {
        DateTime {}
    }
}

impl BarModule for DateTime {
    fn refresh(&mut self) -> Result<String, Error> {
        let now = Local::now();
        Ok(now.format("%a. %-e %B %Y, %-kh%M").to_string())
    }
}
