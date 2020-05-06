// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::nl_data::{self, State};
use crate::{BarModule, Config};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const TICK_RATE: Duration = Duration::from_millis(500);

pub struct Wireless<'a> {
    config: &'a Config,
    handle: JoinHandle<Result<(), Error>>,
    receiver: Receiver<State>,
    prev_data: Option<State>,
}

impl<'a> Wireless<'a> {
    pub fn with_config(config: &'a Config) -> Self {
        let (tx, rx) = mpsc::channel();
        let tick = match &config.wireless_tick {
            Some(ms) => Duration::from_millis(*ms as u64),
            None => TICK_RATE,
        };
        let handle = thread::spawn(move || -> Result<(), Error> {
            run(tick, tx)?;
            Ok(())
        });
        Wireless {
            handle,
            receiver: rx,
            config,
            prev_data: None,
        }
    }

    pub fn data(&self) -> Option<State> {
        self.receiver.try_iter().last()
    }
}

impl<'a> BarModule for Wireless<'a> {
    fn refresh(&mut self) -> Result<String, Error> {
        if let Some(state) = self.data() {
            self.prev_data = Some(state);
        }
        let icon = if let Some(state) = &self.prev_data {
            if let State::Connected(data) = state {
                if let Some(strength) = data.signal {
                    match strength {
                        0 => "󰤯",
                        1..=25 => "󰤟",
                        26..=50 => "󰤢",
                        51..=75 => "󰤥",
                        _ => "󰤨",
                    }
                } else {
                    "󰤫"
                }
            } else {
                "󰤮"
            }
        } else {
            "󰤫"
        };
        Ok(format!(
            "{}{}{}",
            self.config.icon_font, icon, self.config.default_font
        ))
    }
}

fn run(tick: Duration, tx: Sender<State>) -> Result<(), Error> {
    loop {
        tx.send(nl_data::data())?;
        thread::sleep(tick);
    }
}
