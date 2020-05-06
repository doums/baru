// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use bar::pulse::Pulse;
use bar::{Bar, Config, ModuleConfig};
use std::env;
use std::fs;
use std::io::Error;
use std::process;
use std::thread;
use std::time::Duration;

const TICK_RATE: Duration = Duration::from_millis(250);

fn print_out_err(message: &str) {
    println!("{}", message);
    eprintln!("{}", message);
}

fn main() -> Result<(), Error> {
    let home = env::var("HOME").unwrap_or_else(|err| {
        print_out_err(&format!("bar: environment variable HOME, {}", err));
        process::exit(1);
    });
    let content = fs::read_to_string(home + "/.config/bar/bar.yaml").unwrap_or_else(|err| {
        print_out_err(&format!(
            "bar: error while reading the config file, {}",
            err
        ));
        process::exit(1);
    });
    let config: Config = serde_yaml::from_str(&content).unwrap_or_else(|err| {
        print_out_err(&format!(
            "bar: error while deserializing the config file, {}",
            err
        ));
        process::exit(1);
    });
    let tick = match config.tick {
        Some(ms) => Duration::from_millis(ms as u64),
        None => TICK_RATE,
    };
    let pulse = Pulse::new(&config);
    let mut bar = Bar::with_config(&config, &pulse).unwrap_or_else(|err| {
        print_out_err(&format!("bar: {}", err));
        process::exit(1);
    });
    loop {
        bar.update().unwrap_or_else(|err| {
            print_out_err(&format!("bar: {}", err));
            process::exit(1);
        });
        thread::sleep(tick);
    }
}
