// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::http::HTTP_CLIENT;
use crate::module::{Bar, RunPtr};
use crate::{Config as MainConfig, ModuleMsg};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};
use tracing::{debug, error, instrument, trace, warn};

const PLACEHOLDER: &str = "-";
const TICK_RATE: Duration = Duration::from_secs(120);
const OPENWEATHER_API: &str = "https://api.openweathermap.org/data/2.5/weather";
const LABEL: &str = "wtr";
const FORMAT: &str = "%v";
const DEFAULT_W_ICON: &str = "*";
const DEFAULT_LOCATION: Location = Location::Coordinates(Coord {
    lat: 42.38,
    lon: 8.94,
});

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
enum IconSet {
    DayOnly(String),
    DayAndNight((String, String)),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct WeatherIcons {
    clear_sky: Option<IconSet>,
    partly_cloudy: Option<IconSet>,
    cloudy: Option<IconSet>,
    very_cloudy: Option<IconSet>,
    shower_rain: Option<IconSet>,
    rain: Option<IconSet>,
    thunderstorm: Option<IconSet>,
    snow: Option<IconSet>,
    mist: Option<IconSet>,
    default: Option<String>,
}

impl WeatherIcons {
    fn icon(&self, code: u32) -> &Option<IconSet> {
        match code {
            1 => &self.clear_sky,
            2 => &self.partly_cloudy,
            3 => &self.cloudy,
            4 => &self.very_cloudy,
            9 => &self.shower_rain,
            10 => &self.rain,
            11 => &self.thunderstorm,
            13 => &self.snow,
            50 => &self.mist,
            _ => &self.clear_sky,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "snake_case")]
enum Unit {
    Standard,
    #[default]
    Metric,
    Imperial,
}

impl Unit {
    /// Get the corresponding value for the OpenWeather API `units` URL parameter.
    /// see https://openweathermap.org/current#data
    fn to_api(&self) -> &str {
        match self {
            Unit::Standard => "standard",
            Unit::Metric => "metric",
            Unit::Imperial => "imperial",
        }
    }

    fn temp_symbol(&self) -> &str {
        match self {
            Unit::Standard => "K",
            Unit::Metric => "°",   // "°C"
            Unit::Imperial => "°", // "°F"
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Coord {
    lat: f32,
    lon: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
enum Location {
    /// deprecated - city name, zip-code or city ID
    City(String),
    /// Latitude and longitude
    Coordinates(Coord),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    location: Location,
    api_key: String,
    unit: Option<Unit>,
    // two-letter language code
    lang: Option<String>,
    icons: Option<WeatherIcons>,
    text_mode: Option<bool>,
    // Update interval in seconds
    tick: Option<u32>,
    placeholder: Option<String>,
    label: Option<String>,
    format: Option<String>,
}

#[derive(Debug)]
pub struct InternalConfig<'a> {
    location: Location,
    api_key: String,
    unit: Unit,
    lang: Option<&'a str>,
    icons: Option<WeatherIcons>,
    text_mode: bool,
    tick: Duration,
    label: &'a str,
}

impl<'a> Default for InternalConfig<'a> {
    fn default() -> Self {
        InternalConfig {
            location: DEFAULT_LOCATION,
            api_key: String::new(),
            unit: Unit::default(),
            lang: None,
            icons: None,
            text_mode: true,
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
                location: c.location.to_owned(),
                api_key: c.api_key.to_owned(),
                unit: c.unit.to_owned().unwrap_or_default(),
                lang: c.lang.as_deref(),
                icons: c.icons.to_owned(),
                text_mode: c.icons.is_none() || c.text_mode.is_some_and(|b| b),
                tick: c.tick.map_or(TICK_RATE, |t| Duration::from_secs(t as u64)),
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

fn build_url(config: &InternalConfig) -> String {
    let location = match &config.location {
        Location::City(city) => format!("q={city}"),
        Location::Coordinates(Coord { lat, lon }) => format!("lat={lat}&lon={lon}"),
    };
    let mut url = format!(
        "{OPENWEATHER_API}?{location}&units={}&appid={}",
        config.unit.to_api(),
        config.api_key,
    );
    if let Some(lang) = config.lang {
        url.push_str(&format!("&lang={}", lang));
    }
    url
}

fn get_icon<'icon_cfg>(icon: &str, icons: &'icon_cfg WeatherIcons) -> &'icon_cfg str {
    let default = icons.default.as_deref().unwrap_or(DEFAULT_W_ICON);
    let code = icon[0..2]
        .parse::<u32>()
        .inspect_err(|e| error!("failed to parse weather code: {}", e))
        .unwrap_or(99);
    icons.icon(code).as_ref().map_or(default, |i| match i {
        IconSet::DayOnly(icon) => icon,
        IconSet::DayAndNight((day, night)) => match icon.ends_with('d') {
            true => day,
            false => night,
        },
    })
}

fn get_output(json: JsonResponse, config: &InternalConfig) -> String {
    let temp = json.main.temp.round() as u32;
    let t_symbol = config.unit.temp_symbol();
    let data = json
        .weather
        .first()
        .ok_or("no weather data")
        .inspect_err(|_| warn!("no weather data in response"));
    if config.text_mode {
        let desc = data.map_or("N/A", |w| w.description.as_str());
        return format!("{desc} {temp}{t_symbol}");
    }
    let icon_code = data.map_or(DEFAULT_W_ICON, |w| w.icon.as_str());
    trace!("icon code: {}", icon_code);
    let icon = config
        .icons
        .as_ref()
        .map_or(DEFAULT_W_ICON, |i| get_icon(icon_code, i));
    format!("{icon} {temp}{t_symbol}")
}

#[instrument(skip_all)]
pub fn run(key: char, main_config: MainConfig, tx: Sender<ModuleMsg>) -> Result<(), Error> {
    let config = InternalConfig::try_from(&main_config)?;
    debug!("{:#?}", config);
    let mut iteration_start: Instant;
    let mut iteration_end: Duration;
    let url = build_url(&config);
    debug!("openweather URL: {}", url);
    loop {
        iteration_start = Instant::now();
        let response = HTTP_CLIENT
            .get(&url)
            .send()
            .inspect_err(|e| error!("request failed, {}", e))
            .ok();
        if let Some(res) = response {
            let output = res
                .json::<JsonResponse>()
                .inspect_err(|e| error!("failed to parse response body, {}", e))
                .inspect(|json| trace!("response body: {:#?}", json))
                .ok()
                .map(|json| get_output(json, &config));
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

#[derive(Default, Debug, Clone, Deserialize)]
struct JsonResponse {
    weather: Vec<WeatherData>,
    main: MainData,
}

#[derive(Default, Debug, Clone, Deserialize)]
struct WeatherData {
    description: String,
    icon: String,
}

#[derive(Default, Debug, Clone, Deserialize)]
struct MainData {
    temp: f64,
}
