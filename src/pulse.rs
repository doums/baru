// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::Config;
use libpulse_binding as pulse;
use pulse::callbacks::ListResult;
use pulse::context::introspect::{SinkInfo, SourceInfo};
use pulse::context::subscribe::{subscription_masks, Facility};
use pulse::context::{flags, Context, State};
use pulse::mainloop::standard::{IterateResult, Mainloop};
use pulse::proplist::Proplist;
use pulse::volume::ChannelVolumes;
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const PULSE_RATE: Duration = Duration::from_millis(16);
const SINK_INDEX: u32 = 0;
const SOURCE_INDEX: u32 = 0;

#[derive(Copy, Clone, Debug)]
pub struct PulseData(pub i32, pub bool);

struct IntroInfo {
    volume: ChannelVolumes,
    muted: bool,
}

impl From<&SinkInfo<'_>> for IntroInfo {
    fn from(sink_info: &SinkInfo) -> Self {
        IntroInfo {
            volume: sink_info.volume,
            muted: sink_info.mute,
        }
    }
}

impl From<&SourceInfo<'_>> for IntroInfo {
    fn from(source_info: &SourceInfo) -> Self {
        IntroInfo {
            volume: source_info.volume,
            muted: source_info.mute,
        }
    }
}

pub struct Pulse(
    JoinHandle<Result<(), Error>>,
    Receiver<PulseData>,
    Receiver<PulseData>,
);

impl Pulse {
    pub fn new<'a>(config: &'a Config) -> Result<Self, Error> {
        let (out_tx, out_rx) = mpsc::channel();
        let (in_tx, in_rx) = mpsc::channel();
        let tick = match &config.pulse_tick {
            Some(val) => Duration::from_millis(*val as u64),
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
            run(tick, sink_index, source_index, out_tx, in_tx)?;
            Ok(())
        })?;
        Ok(Pulse(handle, out_rx, in_rx))
    }

    pub fn output_data(&self) -> Option<PulseData> {
        self.1.try_iter().last()
    }

    pub fn input_data(&self) -> Option<PulseData> {
        self.2.try_iter().last()
    }
}

fn run(
    tick: Duration,
    sink: u32,
    source: u32,
    out_tx: Sender<PulseData>,
    in_tx: Sender<PulseData>,
) -> Result<(), Error> {
    let mut proplist = Proplist::new().unwrap();
    proplist
        .set_str(pulse::proplist::properties::APPLICATION_NAME, "Bar")
        .unwrap();
    let mainloop = Rc::new(RefCell::new(
        Mainloop::new().expect("in pulse module, failed to create mainloop"),
    ));
    let context = Rc::new(RefCell::new(
        Context::new_with_proplist(mainloop.borrow().deref(), "BarContext", &proplist)
            .expect("in pulse module, failed to create new context"),
    ));
    context
        .borrow_mut()
        .connect(None, flags::NOFAIL, None)
        .unwrap();
    loop {
        match mainloop.borrow_mut().iterate(false) {
            IterateResult::Quit(_) | IterateResult::Err(_) => {
                eprintln!("in pulse module, a mainloop iteration failed");
                return Err(Error::from("in pulse module, a mainloop iteration failed"));
            }
            _ => {}
        }
        match context.borrow().get_state() {
            State::Ready => {
                break;
            }
            State::Failed | State::Terminated => {
                eprintln!("in pulse module, context state failed");
                return Err(Error::from("in pulse module, context state failed"));
            }
            _ => {}
        }
        thread::sleep(tick);
    }
    let introspector = &context.borrow().introspect();
    // initial introspections
    let out_tx1 = Sender::clone(&out_tx);
    introspector.get_sink_info_by_index(sink, move |l| {
        if let Some(info) = parse_sink_info(l) {
            out_tx1
                .send(info)
                .expect("in pulse module, failed to send sink data");
        }
    });
    let in_tx1 = Sender::clone(&in_tx);
    introspector.get_source_info_by_index(source, move |l| {
        if let Some(info) = parse_source_info(l) {
            in_tx1
                .send(info)
                .expect("in pulse module, failed to send source data");
        }
    });
    // subscribed instrospections
    let interest = subscription_masks::SINK | subscription_masks::SOURCE;
    let introspector = context.borrow().introspect();
    context.borrow_mut().subscribe(interest, |_| {});
    context
        .borrow_mut()
        .set_subscribe_callback(Some(Box::new(move |facility_opt, _, _| {
            if let Some(facility) = facility_opt {
                match facility {
                    Facility::Sink => {
                        let tx1 = Sender::clone(&out_tx);
                        introspector.get_sink_info_by_index(sink, move |l| {
                            if let Some(info) = parse_sink_info(l) {
                                tx1.send(info)
                                    .expect("in pulse module, failed to send sink data");
                            }
                        });
                    }
                    Facility::Source => {
                        let tx1 = Sender::clone(&in_tx);
                        introspector.get_source_info_by_index(source, move |l| {
                            if let Some(info) = parse_source_info(l) {
                                tx1.send(info)
                                    .expect("in pulse module, failed to send source data");
                            }
                        });
                    }
                    _ => {}
                }
            }
        })));
    // mainloop
    loop {
        match mainloop.borrow_mut().iterate(false) {
            IterateResult::Quit(_) | IterateResult::Err(_) => {
                eprintln!("in pulse module, a mainloop iteration failed");
                return Err(Error::from("in pulse module, a mainloop iteration failed"));
            }
            _ => {}
        }
        thread::sleep(tick);
    }
}

fn parse_sink_info(list: ListResult<&SinkInfo>) -> Option<PulseData> {
    match list {
        ListResult::Item(item) => {
            return match parse_info(&IntroInfo::from(item)) {
                Ok(data) => Some(data),
                Err(err) => {
                    eprintln!("in pulse module, sink, {}", err);
                    return None;
                }
            };
        }
        ListResult::Error => {
            eprintln!("in pulse module, failed to get sink info");
            return None;
        }
        _ => None,
    }
}

fn parse_source_info(list: ListResult<&SourceInfo>) -> Option<PulseData> {
    match list {
        ListResult::Item(item) => {
            return match parse_info(&IntroInfo::from(item)) {
                Ok(data) => Some(data),
                Err(err) => {
                    eprintln!("in pulse module, source, {}", err);
                    return None;
                }
            };
        }
        ListResult::Error => {
            eprintln!("in pulse module, failed to get source info");
            return None;
        }
        _ => None,
    }
}

fn parse_info(info: &IntroInfo) -> Result<PulseData, Error> {
    let mut average_str = info.volume.avg().print();
    let average_opt = average_str.pop();
    if let Some(char) = average_opt {
        if char != '%' {
            return Err(Error::from("failed to parse volume, char \"%\" expected"));
        }
    } else {
        return Err(Error::from("failed to parse volume, char \"%\" expected"));
    }
    match average_str.trim().parse::<i32>() {
        Ok(average) => Ok(PulseData(average, info.muted)),
        Err(err) => Err(Error::new(format!("failed to parse volume: {}", err))),
    }
}
