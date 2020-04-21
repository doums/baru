// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use bar::Bar;
use std::io::Error;
use std::process;
use std::thread;
use std::time::Duration;

const TICK_RATE: Duration = Duration::from_millis(250);

fn main() -> Result<(), Error> {
    let mut bar = Bar::new().unwrap_or_else(|err| {
        println!("bar error: {}", err);
        process::exit(1);
    });
    loop {
        bar.update().unwrap_or_else(|err| {
            println!("bar error: {}", err);
            process::exit(1);
        });
        thread::sleep(TICK_RATE);
    }
}
