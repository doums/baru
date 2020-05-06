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
    fn markup(&self) -> char {
        'd'
    }

    fn refresh(&mut self) -> Result<String, Error> {
        let now = Local::now();
        Ok(now.format("%a. %-e %B %Y, %-kh%M").to_string())
    }
}
