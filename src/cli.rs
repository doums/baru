// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, ValueEnum)]
pub enum Logs {
    Off,
    Stdout,
    File,
}

#[derive(Parser, Serialize, Deserialize, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Enable app logs
    #[arg(short, long)]
    pub logs: Option<Logs>,
}
