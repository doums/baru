// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::nl_data::{self, WiredState};
use crate::{BarModule, Config as MainConfig};
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const TICK_RATE: Duration = Duration::from_millis(1000);
const INTERFACE: &str = "enp0s31f6";

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    tick: Option<u32>,
    interface: Option<String>,
    discrete: Option<bool>,
}

pub struct Wired<'a> {
    config: &'a MainConfig,
    handle: JoinHandle<Result<(), Error>>,
    receiver: Receiver<WiredState>,
    prev_data: Option<WiredState>,
    discrete: bool,
}

impl<'a> Wired<'a> {
    pub fn with_config(config: &'a MainConfig) -> Result<Self, Error> {
        let (tx, rx) = mpsc::channel();
        let mut tick = TICK_RATE;
        let mut interface = INTERFACE.to_string();
        let mut discrete = false;
        if let Some(c) = &config.wired {
            if let Some(t) = c.tick {
                tick = Duration::from_millis(t as u64)
            }
            if let Some(i) = &c.interface {
                interface = i.clone()
            }
            if let Some(b) = c.discrete {
                discrete = b;
            }
        };
        let builder = thread::Builder::new().name("wired_mod".into());
        let handle = builder.spawn(move || -> Result<(), Error> {
            run(tick, tx, interface)?;
            Ok(())
        })?;
        Ok(Wired {
            handle,
            receiver: rx,
            config,
            prev_data: None,
            discrete,
        })
    }

    pub fn data(&self) -> Option<WiredState> {
        self.receiver.try_iter().last()
    }
}

impl<'a> BarModule for Wired<'a> {
    fn refresh(&mut self) -> Result<String, Error> {
        if let Some(state) = self.data() {
            self.prev_data = Some(state);
        }
        let mut icon = "󰈂";
        if let Some(state) = &self.prev_data {
            if let WiredState::Connected = state {
                icon = "󰈁";
            }
        }
        if self.discrete && icon == "󰈂" {
            return Ok("".to_string());
        }
        Ok(format!(
            "{}{}{}",
            self.config.icon_font, icon, self.config.default_font
        ))
    }
}

fn run(tick: Duration, tx: Sender<WiredState>, interface: String) -> Result<(), Error> {
    loop {
        tx.send(nl_data::wired_data(&interface))?;
        thread::sleep(tick);
    }
}
