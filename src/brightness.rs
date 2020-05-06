use crate::error::Error;
use crate::{read_and_parse, BarModule, Config};

const BACKLIGHT_PATH: &'static str =
    "/sys/devices/pci0000:00/0000:00:02.0/drm/card0/card0-eDP-1/intel_backlight";

#[derive(Debug)]
pub struct Brightness<'a> {
    backlight: &'a str,
    config: &'a Config,
}

impl<'a> Brightness<'a> {
    pub fn with_config(config: &'a Config) -> Self {
        Brightness {
            backlight: match &config.backlight {
                Some(val) => &val,
                None => BACKLIGHT_PATH,
            },
            config,
        }
    }
}

impl<'a> BarModule for Brightness<'a> {
    fn markup(&self) -> char {
        'b'
    }

    fn refresh(&mut self) -> Result<String, Error> {
        let brightness = read_and_parse(&format!("{}/actual_brightness", self.backlight))?;
        let max_brightness = read_and_parse(&format!("{}/max_brightness", self.backlight))?;
        let percentage = 100 * brightness / max_brightness;
        Ok(format!(
            "{:3}% {}󰃟{}",
            percentage, self.config.icon_font, self.config.default_font
        ))
    }
}
