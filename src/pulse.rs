// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::Config;
use crate::error::Error;
use anyhow::Result;
use once_cell::sync::OnceCell;
use std::os::raw::c_char;
use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::{ffi::CString, ptr};
use tracing::{error, info, instrument, warn};

const PULSE_RATE: u32 = 50_000_000; // in nanosecond

pub type Callback = extern "C" fn(*const CallbackContext, u32, bool);

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct PulseData(pub u32, pub bool);

#[repr(C)]
pub struct CallbackContext(Sender<PulseData>, Sender<PulseData>);

/// 0: sink, 1: source
pub struct Pulse(Receiver<PulseData>, Receiver<PulseData>);

pub static PULSE: OnceCell<Mutex<Pulse>> = OnceCell::new();

#[instrument(skip_all)]
pub fn init(config: &Config) -> Result<JoinHandle<Result<(), Error>>> {
    let (pulse, handle) = Pulse::new(config).inspect_err(|e| {
        error!("error pulse module: {}", e);
    })?;

    info!("initializing pulse module");
    PULSE
        .set(Mutex::new(pulse))
        .inspect_err(|_| {
            warn!("error initializing pulse module: already initialized");
        })
        .ok();
    Ok(handle)
}

impl Pulse {
    #[instrument(skip_all)]
    pub fn new(config: &Config) -> Result<(Self, JoinHandle<Result<(), Error>>), Error> {
        let (sink_tx, sink_rx) = mpsc::channel();
        let (source_tx, source_rx) = mpsc::channel();
        let tick = match config.pulse_tick {
            Some(val) => val * 1e6 as u32,
            None => PULSE_RATE,
        };
        let mut sink_name = None;
        let mut source_name = None;
        if let Some(c) = &config.sound {
            sink_name = c.sink_name.clone();
        }
        if let Some(c) = &config.mic {
            source_name = c.source_name.clone();
        }
        let builder = thread::Builder::new().name("pulse".into());
        let handle = builder.spawn(move || -> Result<(), Error> {
            let cb_context = CallbackContext(sink_tx, source_tx);
            pulse_run(
                tick,
                sink_name,
                source_name,
                &cb_context,
                sink_cb,
                source_cb,
            );
            info!("pulse module stopped");
            Ok(())
        })?;
        Ok((Pulse(sink_rx, source_rx), handle))
    }

    pub fn sink_data(&self) -> Option<PulseData> {
        self.0.try_iter().last()
    }

    pub fn source_data(&self) -> Option<PulseData> {
        self.1.try_iter().last()
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

#[link(name = "audio", kind = "static")]
unsafe extern "C" {
    fn run(
        run: *const bool,
        tick: u32,
        sink_name: *const c_char,
        source_name: *const c_char,
        cb_context: *const CallbackContext,
        sink_cb: Callback,
        source_cb: Callback,
    );
}

pub fn pulse_run(
    tick: u32,
    sink_name: Option<String>,
    source_name: Option<String>,
    callback_context: &CallbackContext,
    sink_cb: Callback,
    source_cb: Callback,
) {
    let context_ptr: *const CallbackContext = callback_context;
    let mut ptr_sink = ptr::null();
    let mut ptr_source = ptr::null();
    let c_string_sink;
    let c_string_source;
    if let Some(s) = sink_name {
        c_string_sink = CString::new(s).expect("CString::new failed");
        ptr_sink = c_string_sink.as_ptr();
    };
    if let Some(s) = source_name {
        c_string_source = CString::new(s).expect("CString::new failed");
        ptr_source = c_string_source.as_ptr();
    };

    unsafe {
        run(
            super::RUN.as_ptr(),
            tick,
            ptr_sink,
            ptr_source,
            context_ptr,
            sink_cb,
            source_cb,
        );
    }
}
