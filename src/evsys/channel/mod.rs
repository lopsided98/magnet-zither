// SPDX-License-Identifier: GPL-3.0-or-later
use crate::pac;

mod reg;

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum User {
    DmacCh0 = 0x0,
    DmacCh1 = 0x1,
    DmacCh2 = 0x2,
    DmacCh3 = 0x3,
    Tc3 = 0x12,
    Tc4 = 0x13,
    Tc5 = 0x14,
    Tc6 = 0x15,
    Tc7 = 0x16,
}

pub type EdgeSelection = pac::evsys::channel::EDGSEL_A;

pub type Path = pac::evsys::channel::PATH_A;

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum EventGenerator {
    None = 0x0,
    EicExtint8 = 0x14,
    AcComp0 = 0x44,
    AcComp1 = 0x45,
    AcWin0 = 0x46,
}

pub struct Channel<const ID: u8> {
    reg: reg::RegisterBlock<ID>,
}

impl<const ID: u8> Channel<ID> {
    pub(super) fn new() -> Self {
        Self {
            reg: reg::RegisterBlock::new(),
        }
    }

    pub fn user(&self, user: User) {
        self.reg
            .user()
            .write(|w| unsafe { w.user().bits(user as u8) });
    }

    pub fn config(&self, path: Path, event_generator: EventGenerator) {
        self.reg.channel().write(|w| {
            let w = w.path().variant(path);
            unsafe { w.evgen().bits(event_generator as u8) }
        });
    }
}
