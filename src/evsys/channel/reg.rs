// SPDX-License-Identifier: GPL-3.0-or-later
use crate::const_assert::const_assert;
use pac::evsys;
use pac::evsys::channel;
use pac::evsys::user;

use crate::pac;

trait Register<const ID: u8> {
    fn evsys(&self) -> pac::EVSYS;
}

pub(super) struct User<'a, const ID: u8> {
    reg: &'a evsys::USER,
}

impl<'a, const ID: u8> User<'a, ID> {
    fn new(reg: &'a evsys::USER) -> Self {
        Self { reg }
    }

    pub fn write<F>(&self, f: F)
    where
        F: FnOnce(&mut user::W) -> &mut user::W,
    {
        self.reg.write(|w| {
            let w = f(w);
            unsafe { w.channel().bits(ID + 1) }
        })
    }
}

pub(super) struct Channel<'a, const ID: u8> {
    reg: &'a evsys::CHANNEL,
}

impl<'a, const ID: u8> Channel<'a, ID> {
    fn new(reg: &'a evsys::CHANNEL) -> Self {
        Self { reg }
    }

    pub fn write<F>(&self, f: F)
    where
        F: FnOnce(&mut channel::W) -> &mut channel::W,
    {
        self.reg.write(|w| {
            let w = f(w);
            unsafe { w.channel().bits(ID) }
        })
    }
}

pub(super) struct RegisterBlock<const ID: u8> {
    evsys: pac::EVSYS,
}

impl<const ID: u8> RegisterBlock<ID> {
    const_assert!(ID < super::super::EventSystem::NUM_CHANNELS);

    pub fn new() -> Self {
        Self {
            evsys: unsafe { pac::Peripherals::steal() }.EVSYS,
        }
    }

    pub fn user(&self) -> User<ID> {
        User::new(&self.evsys.user)
    }

    pub fn channel(&self) -> Channel<ID> {
        Channel::new(&self.evsys.channel)
    }
}
