// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use std::process::Command;
use std::str;
use std::sync::mpsc::TryRecvError;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const TICK_RATE: Duration = Duration::from_millis(250);

pub struct Sound(JoinHandle<Result<(), Error>>, Receiver<PulseAudio>);

pub struct PulseAudio {
    in_volume: Option<i32>,
    out_volume: Option<i32>,
    in_muted: bool,
    out_muted: bool,
}

impl PulseAudio {
    fn new(
        in_volume: Option<i32>,
        out_volume: Option<i32>,
        in_muted: bool,
        out_muted: bool,
    ) -> Self {
        PulseAudio {
            in_volume,
            out_volume,
            in_muted,
            out_muted,
        }
    }
}

impl Sound {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let handle = thread::spawn(move || -> Result<(), Error> {
            run(tx)?;
            Ok(())
        });
        Sound(handle, rx)
    }

    pub fn data(self) -> Result<PulseAudio, TryRecvError> {
        self.1.try_recv()
    }
}

fn run(tx: Sender<PulseAudio>) -> Result<(), Error> {
    loop {
        let output = Command::new("pacmd")
            .arg("list-sinks")
            .output()
            .map_err(|_err| "an error occurred while running \"pacmd\"")?;
        if !output.status.success() {
            let pacmd_stderr = str::from_utf8(&output.stderr)?;
            return Err(Error::new(format!("\"pacmd\" error: {}", pacmd_stderr)));
        } else {
            let in_volume = Some(56);
            let out_volume = Some(11);
            let in_muted = false;
            let out_muted = true;
            let response = PulseAudio::new(in_volume, out_volume, in_muted, out_muted);
            tx.send(response)?
        }
        thread::sleep(TICK_RATE);
    }
}
