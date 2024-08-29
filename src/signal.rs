// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::RUN;

use anyhow::Result;
use signal_hook::consts::{SIGINT, SIGQUIT, SIGTERM};
use signal_hook::iterator::Signals;
use std::process::exit;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;
use tracing::info;

const SIGNALS: [i32; 3] = [SIGINT, SIGTERM, SIGQUIT];
const EXIT_TIMEOUT: Duration = Duration::from_millis(500);

pub fn catch_signals() -> Result<()> {
    let mut signals = Signals::new(SIGNALS)?;
    let builder = thread::Builder::new().name("signal_handler".into());

    builder.spawn(move || {
        if let Some(sig) = signals.forever().next() {
            match sig {
                SIGINT => info!("received {sig}:SIGINT"),
                SIGTERM => info!("received {sig}:SIGTERM"),
                SIGQUIT => info!("received {sig}:SIGQUIT"),
                _ => {}
            }
            RUN.store(false, Ordering::Relaxed);
            // wait for the main app thread to close and exit
            // if it takes too long force exit
            thread::sleep(EXIT_TIMEOUT);
            info!("force exit");
            exit(0);
        }
    })?;
    Ok(())
}
