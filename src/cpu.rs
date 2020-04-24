// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub struct CpuData(pub i32, pub i32);

pub struct Cpu(JoinHandle<Result<(), Error>>, Receiver<CpuData>);

impl Cpu {
    pub fn new(tick: Duration, file: &str) -> Self {
        let (tx, rx) = mpsc::channel();
        let file = file.to_string();
        let handle = thread::spawn(move || -> Result<(), Error> {
            run(tick, file, tx)?;
            Ok(())
        });
        Cpu(handle, rx)
    }

    pub fn data(&self) -> Option<CpuData> {
        self.1.try_iter().last()
    }
}

fn run(tick: Duration, file: String, tx: Sender<CpuData>) -> Result<(), Error> {
    loop {
        let proc_stat = File::open(&file)?;
        let mut reader = BufReader::new(proc_stat);
        let mut buf = String::new();
        reader.read_line(&mut buf)?;
        let mut data = buf.split_whitespace();
        data.next();
        let times: Vec<i32> = data
            .map(|n| {
                n.parse::<i32>()
                    .expect(&format!("error while parsing the file \"{}\"", file))
            })
            .collect();
        let idle = times[3] + times[4];
        let total = times.iter().fold(0, |acc, i| acc + i);
        tx.send(CpuData(total, idle))?;
        thread::sleep(tick);
    }
}
