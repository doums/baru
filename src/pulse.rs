// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use libpulse_binding as pulse;
use pulse::callbacks::ListResult;
use pulse::context::subscribe::subscription_masks;
use pulse::context::{flags, Context, State};
use pulse::def::Retval;
use pulse::mainloop::standard::IterateResult;
use pulse::mainloop::standard::Mainloop;
use pulse::proplist::Proplist;
use regex::Regex;
use std::cell::RefCell;
use std::ops::Deref;
use std::process::{Command, Output};
use std::rc::Rc;
use std::str;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError::Empty};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const TICK_RATE: Duration = Duration::from_millis(50);

#[derive(Copy, Clone, Debug)]
pub struct OutputData(i32, bool);

pub struct Pulse(JoinHandle<Result<(), Error>>, Receiver<OutputData>);

impl Pulse {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let handle = thread::spawn(move || -> Result<(), Error> {
            run(tx)?;
            Ok(())
        });
        Pulse(handle, rx)
    }

    pub fn data(&self) -> Result<Option<OutputData>, Error> {
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

pub fn run(tx: Sender<OutputData>) -> Result<(), Error> {
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
        thread::sleep(TICK_RATE);
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
    }

    // let interest = subscription_masks::SINK;
    // let mut subscription = context.borrow_mut().subscribe(interest, |_| {});
    // context.borrow_mut().set_subscribe_callback(Some(Box::new(
    // |facility_opt, operation_opt, index| {
    // println!("{:#?}\n{:#?}\n{:#?}\n", facility_opt, operation_opt, index);
    // },
    // )));

    loop {
        thread::sleep(TICK_RATE);
        match mainloop.borrow_mut().iterate(false) {
            IterateResult::Quit(_) | IterateResult::Err(_) => {
                eprintln!("in pulse module, a mainloop iteration failed");
                return Err(Error::new(
                    "in pulse module, a mainloop iteration failed".to_string(),
                ));
            }
            _ => {}
        }
        let introspector = &context.borrow().introspect();
        let tx1 = Sender::clone(&tx);
        introspector.get_sink_info_by_index(0, move |l| match l {
            ListResult::Item(item) => {
                let error = "in pulse module, failed to parse volume";
                let mut average_str = item.volume.avg().print();
                let average_opt = average_str.pop();
                if let Some(char) = average_opt {
                    if char != '%' {
                        eprintln!("{}", error);
                        return;
                    }
                } else {
                    eprintln!("{}", error);
                    return;
                }
                let average = average_str.trim().parse::<i32>().expect(error);
                println!("{}-{}", item.mute, average);
                tx1.send(OutputData(average, item.mute))
                    .expect("in pulse module, failed to send data");
            }
            ListResult::Error => eprintln!("in pulse module, failed to get sink 0"),
            _ => {}
        });
    }
}
