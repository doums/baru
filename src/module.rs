use crate::battery::Battery;
use crate::brightness::Brightness;
use crate::date_time::DateTime;
use crate::error::Error;
use crate::memory::Memory;
use crate::temperature::Temperature;
use crate::Refresh;

#[derive(Debug)]
pub enum Module<'a> {
    DateTime(DateTime),
    Battery(Battery<'a>),
    Brightness(Brightness<'a>),
    // Cpu(Cpu),
    Temperature(Temperature<'a>),
    // Sound(Sound),
    // Mic(Mic),
    // Wireless(Wireless),
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
            _ => Err(Error::new("module unknown".to_string())),
        };
    }
}
