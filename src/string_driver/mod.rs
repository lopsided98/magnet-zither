use atsamd_hal::clock;
use atsamd_hal::time::{Hertz, Nanoseconds, U32Ext};

use atsamd_hal::pac::{PM, TCC0, TCC1, TCC2};
use core::num::Wrapping;
use core::ops::Deref;

mod reg;

enum ChannelState {
    On,
    Off,
}

pub struct Channel<TCC, const ID: u8>
where
    TCC: Deref<Target = crate::pac::tcc0::RegisterBlock>,
{
    reg: reg::RegisterBlock<TCC, ID>,
    /// The frequency of the timer
    timer_period: Nanoseconds,
    off_period: u16,
    on_period: u16,
    state: ChannelState,
}

impl<TCC, const ID: u8> Channel<TCC, ID>
where
    TCC: Deref<Target = crate::pac::tcc0::RegisterBlock>,
{
    unsafe fn new(driver: &StringDriver<TCC>) -> Self {
        Self {
            reg: reg::RegisterBlock::new(&driver.tcc),
            timer_period: driver.timer_period,
            off_period: 0,
            on_period: 0,
            state: ChannelState::On,
        }
    }

    fn next_period(&mut self) -> u16 {
        let (state, period) = match self.state {
            ChannelState::Off => (ChannelState::On, self.on_period),
            ChannelState::On => (ChannelState::Off, self.off_period),
        };
        self.state = state;
        period
    }

    fn update_period(&mut self) {
        let period = self.next_period();
        while self.reg.syncbusy_cc().bit() {}
        self.reg.cc().modify(|r, w| {
            let cc = Wrapping(r.bits() as u16) + Wrapping(period);
            unsafe { w.bits(cc.0 as u32) }
        });
    }

    pub fn on_interrupt(&mut self) {
        if self.reg.intflag_mc().bit() {
            self.update_period();
        }

        self.reg.intflag_mc_clear();
    }
}

impl<TCC, const ID: u8> crate::string::Driver for Channel<TCC, ID>
where
    TCC: Deref<Target = crate::pac::tcc0::RegisterBlock>,
{
    fn set(&mut self, period: Nanoseconds, amplitude: u8, _invert: bool) {
        let period = period.0 / self.timer_period.0;
        let on_period = period * amplitude as u32 / u8::MAX as u32 / 2;
        self.on_period = on_period as u16;
        self.off_period = (period - on_period) as u16;
    }
}

pub struct Channels<TCC: Deref<Target = crate::pac::tcc0::RegisterBlock>>(
    pub Channel<TCC, 0>,
    pub Channel<TCC, 1>,
    pub Channel<TCC, 2>,
    pub Channel<TCC, 3>,
);

pub struct StringDriver<TCC>
where
    TCC: Deref<Target = crate::pac::tcc0::RegisterBlock>,
{
    /// The frequency of the timer
    timer_period: Nanoseconds,
    tcc: TCC,
}

impl<TCC> StringDriver<TCC>
where
    TCC: Deref<Target = crate::pac::tcc0::RegisterBlock>,
{
    pub fn split(self) -> Channels<TCC> {
        unsafe {
            Channels(
                Channel::new(&self),
                Channel::new(&self),
                Channel::new(&self),
                Channel::new(&self),
            )
        }
    }
}

macro_rules! string_driver {
    ($(($TCC:ident, $clock:ident, $apmask:ident, $apbits:ident),)+) => {
        $(

impl StringDriver<$TCC> {
    pub fn new(
        clock: &clock::$clock,
        freq: impl Into<Hertz>,
        tcc: $TCC,
        pm: &mut PM,
    ) -> Self {
        // Power on TCC
        pm.$apmask.modify(|_, w| w.$apbits().set_bit());

        let divider = {
            let divider = (clock.freq().0 / freq.into().0).next_power_of_two();
            match divider {
                1 | 2 | 4 | 8 | 16 | 64 | 256 | 1024 => divider,
                // There are a couple of gaps, so we round up to the next largest
                // divider; we'll need to count twice as many but it will work.
                32 => 64,
                128 => 256,
                512 => 1024,
                // Catch all case; this is lame.  Would be great to detect this
                // and fail at compile time.
                _ => 1024,
            }
        };
        let timer_period = (clock.freq().0 / divider).hz().into();

        let s = Self { timer_period, tcc };

        // Disable TCC
        while s.tcc.syncbusy.read().enable().bit() {}
        s.tcc.ctrla.modify(|_, w| w.enable().clear_bit());
        while s.tcc.syncbusy.read().enable().bit() {}
        // Reset TCC
        s.tcc.ctrla.write(|w| w.swrst().set_bit());
        while s.tcc.syncbusy.read().swrst().bit() {}

        // Set prescaler
        s.tcc.ctrla.write(|w| {
            match divider {
                1 => w.prescaler().div1(),
                2 => w.prescaler().div2(),
                4 => w.prescaler().div4(),
                8 => w.prescaler().div8(),
                16 => w.prescaler().div16(),
                64 => w.prescaler().div64(),
                256 => w.prescaler().div256(),
                1024 => w.prescaler().div1024(),
                _ => unreachable!(),
            }
        });

        // Enable normal frequency (NFRQ) waveform (toggle on compare match)
        s.tcc.wave.write(|w| w.wavegen().nfrq());

        s.tcc.per().write(|w| unsafe { w.bits(u16::MAX as u32) });
        while s.tcc.syncbusy.read().per().bit() {}

        // Enable interrupts
        s.tcc.intenset.write(|w| w.mc0().set_bit()/*.mc1().set_bit().mc2().set_bit().mc3().set_bit()*/);

        // Enable TCC
        s.tcc.ctrla.modify(|_, w| w.enable().set_bit());
        while s.tcc.syncbusy.read().enable().bit() {}

        // Start TCC
        while s.tcc.syncbusy.read().ctrlb().bit() {}
        s.tcc.ctrlbset.write(|w| w.cmd().retrigger());

        s
    }
}

)+}}

string_driver! {
    (TCC0, Tcc0Tcc1Clock, apbcmask, tcc0_),
    (TCC1, Tcc0Tcc1Clock, apbcmask, tcc1_),
    (TCC2, Tcc2Tc3Clock, apbcmask, tcc2_),
}
