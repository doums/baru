// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::new_pulse::pulse_run;
use crate::Config;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

const PULSE_RATE: u32 = 16;
const SINK_INDEX: u32 = 0;
const SOURCE_INDEX: u32 = 0;

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct PulseData(pub u32, pub bool);

#[repr(C)]
pub struct CallbackContext(Sender<PulseData>, Sender<PulseData>);

pub struct Pulse(
    JoinHandle<Result<(), Error>>,
    Receiver<PulseData>,
    Receiver<PulseData>,
);

impl Pulse {
    pub fn new(config: &Config) -> Result<Self, Error> {
        let (sink_tx, sink_rx) = mpsc::channel();
        let (source_tx, source_rx) = mpsc::channel();
        let tick = match config.pulse_tick {
            Some(val) => val,
            None => PULSE_RATE,
        };
        let mut sink_index = SINK_INDEX;
        let mut source_index = SOURCE_INDEX;
        if let Some(c) = &config.sound {
            if let Some(v) = c.index {
                sink_index = v;
            }
        }
        if let Some(c) = &config.mic {
            if let Some(v) = c.index {
                source_index = v;
            }
        }
        let builder = thread::Builder::new().name("pulse_mod".into());
        let handle = builder.spawn(move || -> Result<(), Error> {
            let cb_context = CallbackContext(sink_tx, source_tx);
            pulse_run(
                tick,
                sink_index,
                source_index,
                &cb_context,
                sink_cb,
                source_cb,
            );
            Ok(())
        })?;
        Ok(Pulse(handle, sink_rx, source_rx))
    }

    pub fn sink_data(&self) -> Option<PulseData> {
        self.1.try_iter().last()
    }

    pub fn source_data(&self) -> Option<PulseData> {
        self.2.try_iter().last()
    }
}

extern "C" fn sink_cb(context: *const CallbackContext, volume: u32, mute: bool) {
    unsafe {
        (*context)
            .0
            .send(PulseData(volume, mute))
            .expect("in pulse module, failed to send sink data");
    }
}

extern "C" fn source_cb(context: *const CallbackContext, volume: u32, mute: bool) {
    unsafe {
        (*context)
            .1
            .send(PulseData(volume, mute))
            .expect("in pulse module, failed to send source data");
    }
}
