// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use baru::pulse::Pulse;
use baru::{Baru, Config};
use std::env;
use std::fs;
use std::io::Error;
use std::process;
use std::thread;
use std::time::Duration;

const TICK_RATE: Duration = Duration::from_millis(50);

fn print_out_err(message: &str) {
    println!("{}", message);
    eprintln!("{}", message);
}

fn main() -> Result<(), Error> {
    let home = env::var("HOME").unwrap_or_else(|err| {
        print_out_err(&format!("baru: environment variable HOME, {}", err));
        process::exit(1);
    });
    let config_path = env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| format!("{}/.config", home));
    let content =
        fs::read_to_string(format!("{}/baru/baru.yaml", config_path)).unwrap_or_else(|err| {
            print_out_err(&format!(
                "baru: error while reading the config file, {}",
                err
            ));
            process::exit(1);
        });
    let config: Config = serde_yaml::from_str(&content).unwrap_or_else(|err| {
        print_out_err(&format!(
            "baru: error while deserializing the config file, {}",
            err
        ));
        process::exit(1);
    });
    let tick = match config.tick {
        Some(ms) => Duration::from_millis(ms as u64),
        None => TICK_RATE,
    };
    let pulse = Pulse::new(&config).unwrap_or_else(|err| {
        print_out_err(&format!("baru: error while creating pulse module, {}", err));
        process::exit(1);
    });
    let mut baru = Baru::with_config(&config, &pulse).unwrap_or_else(|err| {
        print_out_err(&format!("baru: {}", err));
        process::exit(1);
    });
    loop {
        baru.update().unwrap_or_else(|err| {
            print_out_err(&format!("baru: {}", err));
            process::exit(1);
        });
        thread::sleep(tick);
    }
}
