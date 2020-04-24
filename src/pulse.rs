// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use libpulse_binding as pulse;
use pulse::callbacks::ListResult;
use pulse::context::introspect::SinkInfo;
use pulse::context::subscribe::subscription_masks;
use pulse::context::{flags, Context, State};
use pulse::mainloop::standard::IterateResult;
use pulse::mainloop::standard::Mainloop;
use pulse::proplist::Proplist;
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

#[derive(Copy, Clone, Debug)]
pub struct OutputData(pub i32, pub bool);

pub struct Pulse(JoinHandle<Result<(), Error>>, Receiver<OutputData>);

impl Pulse {
    pub fn new(tick: Duration) -> Self {
        let (tx, rx) = mpsc::channel();
        let handle = thread::spawn(move || -> Result<(), Error> {
            run(tick, tx)?;
            Ok(())
        });
        Pulse(handle, rx)
    }

    pub fn data(&self) -> Option<OutputData> {
        self.1.try_iter().last()
    }
}

pub fn run(tick: Duration, tx: Sender<OutputData>) -> Result<(), Error> {
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
                return Err(Error::new(
                    "in pulse module, a mainloop iteration failed".to_string(),
                ));
            }
            _ => {}
        }
        match context.borrow().get_state() {
            State::Ready => {
                break;
            }
            State::Failed | State::Terminated => {
                eprintln!("in pulse module, context state failed");
                return Err(Error::new(
                    "in pulse module, context state failed".to_string(),
                ));
            }
            _ => {}
        }
        thread::sleep(tick);
    }
    // initial introspection
    let introspector = &context.borrow().introspect();
    let tx1 = Sender::clone(&tx);
    introspector.get_sink_info_by_index(0, move |l| match parse_sink_info(l) {
        Ok(opt) => {
            if let Some(data) = opt {
                tx1.send(data)
                    .expect("in pulse module, failed to send data");
            }
        }
        Err(err) => {
            eprintln!("in pulse module, {}", err);
            return;
        }
    });
    // subscribed instrospection
    let interest = subscription_masks::SINK;
    let introspector = context.borrow().introspect();
    context.borrow_mut().subscribe(interest, |_| {});
    context
        .borrow_mut()
        .set_subscribe_callback(Some(Box::new(move |_, _, _| {
            let tx1 = Sender::clone(&tx);
            introspector.get_sink_info_by_index(0, move |l| match parse_sink_info(l) {
                Ok(opt) => {
                    if let Some(data) = opt {
                        tx1.send(data)
                            .expect("in pulse module, failed to send data");
                    }
                }
                Err(err) => {
                    eprintln!("in pulse module, {}", err);
                    return;
                }
            });
        })));
    // mainloop
    loop {
        match mainloop.borrow_mut().iterate(false) {
            IterateResult::Quit(_) | IterateResult::Err(_) => {
                eprintln!("in pulse module, a mainloop iteration failed");
                return Err(Error::new(
                    "in pulse module, a mainloop iteration failed".to_string(),
                ));
            }
            _ => {}
        }
        thread::sleep(tick);
    }
}

fn parse_sink_info(list: ListResult<&SinkInfo>) -> Result<Option<OutputData>, Error> {
    match list {
        ListResult::Item(item) => {
            let error = "failed to parse volume";
            let mut average_str = item.volume.avg().print();
            let average_opt = average_str.pop();
            if let Some(char) = average_opt {
                if char != '%' {
                    return Err(Error::new(error.to_string()));
                }
            } else {
                return Err(Error::new(error.to_string()));
            }
            let average = average_str.trim().parse::<i32>().expect(error);
            Ok(Some(OutputData(average, item.mute)))
        }
        ListResult::Error => {
            return Err(Error::new("failed to get sink 0".to_string()));
        }
        _ => Ok(None),
    }
}
