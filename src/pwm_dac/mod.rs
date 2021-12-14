use crate::hal::clock;
use crate::hal::time::{Hertz, Nanoseconds, U32Ext};

use crate::dac::{Dac, DacDmaTrigger};
use crate::pac;
use pac::{PM, TCC0, TCC1, TCC2};
use core::ops::Deref;
use paste::paste;
use seq_macro::seq;

mod reg;

pub struct Channel<TCC, const ID: u8>
where
    TCC: Deref<Target = pac::tcc0::RegisterBlock>,
{
    reg: reg::RegisterBlock<TCC, ID>,
    sample_period: Nanoseconds,
}

impl<TCC, const ID: u8> Channel<TCC, ID>
where
    TCC: Deref<Target = pac::tcc0::RegisterBlock>,
{
    unsafe fn new(driver: &PwmDac<TCC>) -> Self {
        Self {
            reg: reg::RegisterBlock::new(&driver.tcc),
            sample_period: driver.sample_period,
        }
    }
}

impl<TCC, const ID: u8> Dac for Channel<TCC, ID>
where
    Self: DacDmaTrigger,
    TCC: Deref<Target = pac::tcc0::RegisterBlock>,
{
    type Amplitude = u8;
    const MAX_AMPLITUDE: Self::Amplitude = PwmDac::<TCC>::MAX_AMPLITUDE;

    fn set_amplitude(&mut self, amplitude: Self::Amplitude) {
        while self.reg.syncbusy_ccb().bit() {}
        self.reg
            .ccb()
            .write(|w| unsafe { w.ccb().bits(amplitude as u32) });
    }

    fn sample_period(&self) -> Nanoseconds {
        self.sample_period
    }

    fn dma_ptr(&self) -> *mut Self::Amplitude {
        self.reg.ccb().as_ptr().cast()
    }
}

impl<const ID: u8> DacDmaTrigger for Channel<pac::TCC0, ID> {
    const DMA_TRIGGER_SOURCE: samd_dma::TriggerSource = samd_dma::TriggerSource::Tcc0Ovf;
}

impl<const ID: u8> DacDmaTrigger for Channel<pac::TCC1, ID> {
    const DMA_TRIGGER_SOURCE: samd_dma::TriggerSource = samd_dma::TriggerSource::Tcc1Ovf;
}

impl<const ID: u8> DacDmaTrigger for Channel<pac::TCC2, ID> {
    const DMA_TRIGGER_SOURCE: samd_dma::TriggerSource = samd_dma::TriggerSource::Tcc2Ovf;
}

pub struct PwmDac<TCC>
where
    TCC: Deref<Target = pac::tcc0::RegisterBlock>,
{
    tcc: TCC,
    sample_period: Nanoseconds,
}

impl<TCC> PwmDac<TCC>
where
    TCC: Deref<Target = pac::tcc0::RegisterBlock>,
{
    const MAX_AMPLITUDE: u8 = 240;

    pub fn sample_period(&self) -> Nanoseconds {
        self.sample_period
    }
}

macro_rules! pwm_dac {
    ($(($TCC:ident, $channels: literal, $clock:ident, $apmask:ident, $apbits:ident),)+) => {
        paste! {
        $(
        seq!(CH in 0..$channels {

pub struct [<Channels $TCC>] (
    #(pub Channel<$TCC, CH>,)*
);

impl PwmDac<$TCC> {
    pub fn new(
        clock: &clock::$clock,
        freq: impl Into<Hertz>,
        tcc: $TCC,
        pm: &mut PM,
    ) -> Self {
        // Power on TCC
        pm.$apmask.modify(|_, w| w.$apbits().set_bit());

        let divider = {
            // 240 PWM steps and twice the frequency because dual-slope PWM has two overflows per
            // cycle
            let ideal_timer_freq = freq.into().0 * Self::MAX_AMPLITUDE as u32 * 2;

            let divider = (clock.freq().0 / ideal_timer_freq).next_power_of_two();
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

        let sample_period = (clock.freq().0 / divider / Self::MAX_AMPLITUDE as u32 / 2).hz().into();

        let s = Self { tcc, sample_period };

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

        while s.tcc.syncbusy.read().wave().bit() {}
        // Enable dual-slope PWM (DSTOP) and set correct output polarity
        s.tcc.wave.write(|w| w
            .wavegen().dstop()
            .pol0().set_bit()
            .pol1().set_bit()
            .pol2().set_bit()
            .pol3().set_bit()
        );

        // Set PWM duty cycle range
        while s.tcc.syncbusy.read().per().bit() {}
        s.tcc.per().write(|w| unsafe { w.bits(Self::MAX_AMPLITUDE as u32) });

        // Enable TCC
        s.tcc.ctrla.modify(|_, w| w.enable().set_bit());
        while s.tcc.syncbusy.read().enable().bit() {}

        // Start TCC
        while s.tcc.syncbusy.read().ctrlb().bit() {}
        s.tcc.ctrlbset.write(|w| w.cmd().retrigger());

        s
    }

    pub fn split(self) -> [<Channels $TCC>] {
        unsafe {
            [<Channels $TCC>](
                #(Channel::new(&self),)*
            )
        }
    }
}

});)+}}}

pwm_dac! {
    (TCC0, 4, Tcc0Tcc1Clock, apbcmask, tcc0_),
    (TCC1, 2, Tcc0Tcc1Clock, apbcmask, tcc1_),
    (TCC2, 2, Tcc2Tc3Clock, apbcmask, tcc2_),
}
