// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::nl_data::{self, WirelessState};
use crate::{BarModule, Config as MainConfig};
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const TICK_RATE: Duration = Duration::from_millis(500);
const DISPLAY: Display = Display::Signal;
const MAX_ESSID_LEN: usize = 10;
const INTERFACE: &str = "wlp2s0";

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
enum Display {
    Essid,
    Signal,
    IconOnly,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    tick: Option<u32>,
    display: Option<Display>,
    max_essid_len: Option<usize>,
    interface: Option<String>,
}

pub struct Wireless<'a> {
    config: &'a MainConfig,
    handle: JoinHandle<Result<(), Error>>,
    receiver: Receiver<WirelessState>,
    prev_data: Option<WirelessState>,
    display: Display,
    max_essid_len: usize,
}

impl<'a> Wireless<'a> {
    pub fn with_config(config: &'a MainConfig) -> Result<Self, Error> {
        let (tx, rx) = mpsc::channel();
        let mut tick = TICK_RATE;
        let mut display = DISPLAY;
        let mut max_essid_len = MAX_ESSID_LEN;
        let mut interface = INTERFACE.to_string();
        if let Some(c) = &config.wireless {
            if let Some(t) = c.tick {
                tick = Duration::from_millis(t as u64)
            }
            if let Some(d) = &c.display {
                display = *d
            }
            if let Some(m) = c.max_essid_len {
                max_essid_len = m
            }
            if let Some(i) = &c.interface {
                interface = i.clone()
            }
        };
        let builder = thread::Builder::new().name("wireless_mod".into());
        let handle = builder.spawn(move || -> Result<(), Error> {
            run(tick, tx, interface)?;
            Ok(())
        })?;
        Ok(Wireless {
            handle,
            receiver: rx,
            config,
            prev_data: None,
            display,
            max_essid_len,
        })
    }

    pub fn data(&self) -> Option<WirelessState> {
        self.receiver.try_iter().last()
    }
}

impl<'a> BarModule for Wireless<'a> {
    fn refresh(&mut self) -> Result<String, Error> {
        if let Some(state) = self.data() {
            self.prev_data = Some(state);
        }
        let icon;
        let mut essid = "";
        let mut signal = None;
        if let Some(state) = &self.prev_data {
            if let WirelessState::Connected(data) = state {
                if let Some(strength) = data.signal {
                    signal = Some(strength);
                    icon = match strength {
                        0 => "󰤯",
                        1..=25 => "󰤟",
                        26..=50 => "󰤢",
                        51..=75 => "󰤥",
                        _ => "󰤨",
                    }
                } else {
                    icon = "󰤫"
                };
                if let Some(val) = &data.essid {
                    essid = if val.chars().count() > self.max_essid_len {
                        &val[..self.max_essid_len]
                    } else {
                        val
                    }
                }
            } else {
                icon = "󰤮";
            }
        } else {
            icon = "󰤫";
        };
        let icon_format = format!(
            "{}{}{}",
            self.config.icon_font, icon, self.config.default_font
        );
        match self.display {
            Display::IconOnly => Ok(icon_format),
            Display::Essid => Ok(format!("{}{}", essid, icon_format)),
            Display::Signal => {
                if let Some(s) = signal {
                    Ok(format!("{:3}%{}", s, icon_format))
                } else {
                    Ok(format!("    {}", icon_format))
                }
            }
        }
    }
}

fn run(tick: Duration, tx: Sender<WirelessState>, interface: String) -> Result<(), Error> {
    loop {
        tx.send(nl_data::wireless_data(&interface))?;
        thread::sleep(tick);
    }
}
