// SPDX-License-Identifier: GPL-3.0-or-later
use pac::{EVSYS, PM};
use seq_macro::seq;

pub use channel::*;

use crate::pac;

mod channel;

pub struct EventSystem {
    evsys: EVSYS,
}

impl EventSystem {
    pub const NUM_CHANNELS: u8 = 12;

    pub fn new(evsys: EVSYS, pm: &PM) -> Self {
        // Power on EVSYS
        pm.apbcmask.modify(|_, w| w.evsys_().set_bit());

        // Reset EVSYS
        evsys.ctrl.write(|w| w.swrst().set_bit());

        Self { evsys }
    }
}

macro_rules! evsys_channels {
    () => { seq!(CH in 0..12 {

pub struct Channels (
    #(pub Channel<CH>,)*
);

impl EventSystem {
    pub fn split(self) -> Channels {
        Channels (
            #(Channel::new(),)*
        )
    }
}

    });}
}
evsys_channels!();
