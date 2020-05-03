use crate::error::Error;
use crate::pulse::{Pulse, PulseData};
use crate::{Config, Refresh};

pub struct Sound<'a> {
    config: &'a Config,
    pulse: &'a Pulse,
    prev_data: Option<PulseData>,
}

impl<'a> Sound<'a> {
    pub fn with_config(config: &'a Config, pulse: &'a Pulse) -> Self {
        Sound {
            config,
            pulse,
            prev_data: None,
        }
    }
}

impl<'a> Refresh for Sound<'a> {
    fn refresh(&mut self) -> Result<String, Error> {
        let data = self.pulse.output_data();
        if data.is_some() {
            self.prev_data = data;
        }
        let icon;
        let mut color = &self.config.default_color;
        if let Some(info) = self.prev_data {
            if info.1 {
                icon = "󰸈";
            } else {
                icon = match info.0 {
                    0..=9 => "󰕿",
                    10..=40 => "󰖀",
                    _ => "󰕾",
                }
            }
            if info.0 > 150 {
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
            icon = "󰖁";
            Ok(format!(
                "     {}{}{}",
                self.config.icon_font, icon, self.config.default_font
            ))
        }
    }
}
