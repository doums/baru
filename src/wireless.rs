// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::nl_data::{self, Data};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub struct Wireless(JoinHandle<Result<(), Error>>, Receiver<Option<Data>>);

impl Wireless {
    pub fn new(tick: Duration) -> Self {
        let (tx, rx) = mpsc::channel();
        let handle = thread::spawn(move || -> Result<(), Error> {
            run(tick, tx)?;
            Ok(())
        });
        Wireless(handle, rx)
    }

    pub fn data(&self) -> Option<Option<Data>> {
        self.1.try_iter().last()
    }
}

fn run(tick: Duration, tx: Sender<Option<Data>>) -> Result<(), Error> {
    loop {
        tx.send(nl_data::data())?;
        thread::sleep(tick);
    }
}
