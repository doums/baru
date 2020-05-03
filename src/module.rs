// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::battery::Battery;
use crate::brightness::Brightness;
use crate::date_time::DateTime;
use crate::error::Error;
use crate::memory::Memory;
use crate::temperature::Temperature;
use crate::Cpu;
use crate::Mic;
use crate::Refresh;
use crate::Sound;
use crate::Wireless;

pub enum Module<'a> {
    DateTime(DateTime),
    Battery(Battery<'a>),
    Brightness(Brightness<'a>),
    Cpu(Cpu<'a>),
    Temperature(Temperature<'a>),
    Sound(Sound<'a>),
    Mic(Mic<'a>),
    Wireless(Wireless<'a>),
    Memory(Memory<'a>),
}

impl<'a> Refresh for Module<'a> {
    fn refresh(&mut self) -> Result<String, Error> {
        return match self {
            Module::DateTime(m) => m.refresh(),
            Module::Battery(m) => m.refresh(),
            Module::Memory(m) => m.refresh(),
            Module::Brightness(m) => m.refresh(),
            Module::Temperature(m) => m.refresh(),
            Module::Cpu(m) => m.refresh(),
            Module::Wireless(m) => m.refresh(),
            Module::Sound(m) => m.refresh(),
            Module::Mic(m) => m.refresh(),
        };
    }
}
