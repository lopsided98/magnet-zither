use atsamd_hal::clock;
use atsamd_hal::time::{Hertz, U32Ext};

use atsamd_hal::pac::{PM, TCC0, TCC1, TCC2};

macro_rules! pwm_string {
    ($($TYPE:ident: ($TCC:ident, $clock:ident, $apmask:ident, $apbits:ident, $wrapper:ident),)+) => {
        $(

pub struct $TYPE {
    /// The frequency of the timer
    timer_freq: Hertz,
    tcc: $TCC,
}

impl $TYPE {
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
        let timer_freq = (clock.freq().0 / divider).hz();

        let s = Self { timer_freq, tcc };

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

        // Enable dual-slope PWM (DSBOTTOM)
        s.tcc.wave.write(|w| w.wavegen().dsbottom());

        s.tcc.ccb()[0].write(|w| unsafe { w.bits(4000u32) });
        s.configure(398.hz(), 255);

        // Enable TCC
        s.tcc.ctrla.modify(|_, w| w.enable().set_bit());
        while s.tcc.syncbusy.read().enable().bit() {}

        // Start TCC
        while s.tcc.syncbusy.read().ctrlb().bit() {}
        s.tcc.ctrlbset.write(|w| w.cmd().retrigger());

        s
    }

    pub fn configure(&self, freq: impl Into<Hertz>, amplitude: u8) {
        // Lock buffer
        while self.tcc.syncbusy.read().ctrlb().bit() {}
        self.tcc.ctrlbset.write(|w| w.lupd().set_bit());
        while self.tcc.syncbusy.read().ctrlb().bit() {}

        let period = (self.timer_freq.0 / (freq.into().0 * 2)) as u32;
        let cc = period - period * amplitude as u32 / u8::MAX as u32 / 2;

        self.tcc.perb().write(|w| unsafe { w.bits(period) });
        self.tcc.ccb()[0].write(|w| unsafe { w.bits(cc) });

        // Unlock buffer
        while self.tcc.syncbusy.read().ctrlb().bit() {}
        self.tcc.ctrlbclr.write(|w| w.lupd().set_bit());
    }
}

)+}}

pwm_string! {
    Pwm0: (TCC0, Tcc0Tcc1Clock, apbcmask, tcc0_, Pwm0Wrapper),
    Pwm1: (TCC1, Tcc0Tcc1Clock, apbcmask, tcc1_, Pwm1Wrapper),
    Pwm2: (TCC2, Tcc2Tc3Clock, apbcmask, tcc2_, Pwm2Wrapper),
}
