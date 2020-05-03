use crate::error::Error;
use crate::Refresh;
use chrono::prelude::*;

#[derive(Debug)]
pub struct DateTime;

impl DateTime {
    pub fn new() -> Self {
        DateTime {}
    }
}

impl Refresh for DateTime {
    fn refresh(&mut self) -> Result<String, Error> {
        let now = Local::now();
        Ok(now.format("%a. %-e %B %Y, %-kh%M").to_string())
    }
}
