// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use regex::Regex;
use std::process::{Command, Output};
use std::str;
use std::sync::mpsc::TryRecvError::Empty;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const TICK_RATE: Duration = Duration::from_millis(500);

#[derive(Copy, Clone, Debug)]
pub struct PulseAudio {
    pub in_volume: i32,
    pub out_volume: i32,
    pub in_muted: bool,
    pub out_muted: bool,
}

impl PulseAudio {
    fn new(in_volume: i32, out_volume: i32, in_muted: bool, out_muted: bool) -> Self {
        PulseAudio {
            in_volume,
            out_volume,
            in_muted,
            out_muted,
        }
    }
}

pub struct Sound(JoinHandle<Result<(), Error>>, Receiver<PulseAudio>);

impl Sound {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let handle = thread::spawn(move || -> Result<(), Error> {
            run(tx)?;
            Ok(())
        });
        Sound(handle, rx)
    }

    pub fn data(&self) -> Result<Option<PulseAudio>, Error> {
        self.1.try_recv().map_or_else(
            |e| {
                if let Empty = e {
                    return Ok(None);
                } else {
                    return Err(Error::new(format!("error in module volume: {}", e)));
                }
            },
            |v| Ok(Some(v)),
        )
    }
}

fn run(tx: Sender<PulseAudio>) -> Result<(), Error> {
    let r_volume_out = Regex::new(r"front-left:\s*\d*\s*/\s*(\d+)%").unwrap();
    let r_muted_out = Regex::new(r"muted:\s*(no|yes)").unwrap();
    loop {
        let output = Command::new("pacmd")
            .arg("list-sinks")
            .output()
            .map_err(|_err| "an error occurred while running \"pacmd\"")?;
        if output.status.success() {
            let out_volume = search(
                &output,
                &r_volume_out,
                "volume \"percentage\" not found in pacmd output",
            )?
            .parse::<i32>()?;
            let out_muted = search(
                &output,
                &r_muted_out,
                "volume \"muted\" not found in pacmd output",
            )?;
            let out_muted = match out_muted {
                "yes" => true,
                "no" => false,
                _ => return Err(Error::new("format invalid for \"muted\" in pacmd output")),
            };
            let in_volume = 0;
            let in_muted = false;
            tx.send(PulseAudio::new(in_volume, out_volume, in_muted, out_muted))?
        }
        thread::sleep(TICK_RATE);
    }
}

fn search<'a>(output: &'a Output, regex: &'a Regex, err_msg: &'a str) -> Result<&'a str, Error> {
    Ok(regex
        .captures(str::from_utf8(&output.stdout)?)
        .ok_or(Error::new(err_msg.to_string()))?
        .get(1)
        .ok_or(Error::new(err_msg.to_string()))?
        .as_str())
}
