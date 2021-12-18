use crate::hal;
use crate::pac;
use core::ops::Deref;
use hal::clock;
use hal::time::{Nanoseconds, U32Ext};
use num_rational::Ratio;
use pac::{AC, PM, TC3, TC4, TC5};

pub struct AnalogComparator {
    ac: AC,
}

impl AnalogComparator {
    pub fn new(
        _ana_clock: clock::AcAnaClock,
        _dig_clock: clock::AcDigClock,
        ac: AC,
        pm: &PM,
    ) -> Self {
        // Power on AC
        pm.apbcmask.modify(|_, w| w.ac_().set_bit());

        let s = Self { ac };

        // Reset AC
        s.sync();
        s.ac.ctrla.write(|w| w.swrst().set_bit());
        s.sync();

        // Enable event output
        s.ac.evctrl.write(|w| w.compeo0().set_bit());

        // Enable AC
        s.sync();
        s.ac.ctrla.write(|w| w.enable().set_bit());

        // Configure comparator 0
        s.ac.compctrl[0].write(|w| {
            w.flen()
                .maj5()
                .out()
                .sync()
                .muxpos()
                .pin0()
                .muxneg()
                .pin1()
                .intsel()
                .falling()
                .speed()
                .high()
        });

        // Enable comparator 0
        s.sync();
        s.ac.compctrl[0].modify(|_, w| w.enable().set_bit());

        s
    }

    fn sync(&self) {
        while self.ac.statusb.read().syncbusy().bit() {}
    }
}

pub enum Error {
    Overflow,
}

pub struct FrequencyMeter<TC>
where
    TC: Deref<Target = pac::tc3::RegisterBlock>,
{
    tc: TC,
    ns_per_cycle: Ratio<u32>,
}

impl<TC> FrequencyMeter<TC>
where
    TC: Deref<Target = pac::tc3::RegisterBlock>,
{
    fn sync(&self) {
        while self.tc.count16().status.read().syncbusy().bit() {}
    }

    pub fn period_cycles(&self) -> u16 {
        // Request to read CC[0]
        self.tc
            .count16_mut()
            .readreq
            .write(|w| unsafe { w.addr().bits(0x18) }.rreq().set_bit());
        self.sync();
        self.tc.count16().cc[0].read().cc().bits()
    }

    pub fn period_ns(&self) -> Nanoseconds {
        (&self.ns_per_cycle * Ratio::from(self.period_cycles() as u32))
            .round()
            .numer()
            .ns()
    }

    pub fn enable_interrupts(&self) {
        self.tc.count16_mut().intenset.write(|w| w.mc0().set_bit());
    }

    pub fn disable_interrupts(&self) {
        self.tc.count16_mut().intenclr.write(|w| w.mc0().set_bit());
    }

    pub fn on_interrupt(&self) -> Result<(), Error> {
        let flags = self.tc.count16().intflag.read();

        self.tc
            .count16_mut()
            .intflag
            .write(|w| unsafe { w.bits(flags.bits()) });

        if flags.err().bit() {
            Err(Error::Overflow)
        } else {
            Ok(())
        }
    }
}

macro_rules! frequency_meter {
    ($(($TC:ident, $clock:ident, $apmask:ident, $apbits:ident),)+) => {
        $(

impl FrequencyMeter<$TC> {
    pub fn new(
        clock: &clock::$clock,
        tc: $TC,
        pm: &PM,
    ) -> Self {
        // Power on TC
        pm.$apmask.modify(|_, w| w.$apbits().set_bit());

        let ns_per_cycle = Ratio::new(Nanoseconds::from(1.s()).0, clock.freq().0 / 2);
        let s = Self { tc, ns_per_cycle };

        // Disable TCC
        s.sync();
        s.tc.count16_mut().ctrla.modify(|_, w| w.enable().clear_bit());
        s.sync();
        // Reset TCC
        s.tc.count16_mut().ctrla.write(|w| w.swrst().set_bit());
        s.sync();

        s.tc.count16_mut().ctrla.write(|w| w
            .prescaler().div2()
            .mode().count16());

        s.sync();
        s.tc.count16_mut().ctrlc.write(|w| w
            // Enable capture channel 0 for period
            .cpten0().set_bit()
            .cpten1().set_bit());

        s.tc.count16_mut().evctrl.write(|w| w
            // Enable input events
            .tcei().set_bit()
            // Period, pulse-width mode
            .evact().ppw());

        // Enable TC
        s.sync();
        s.tc.count16_mut().ctrla.modify(|_, w| w.enable().set_bit());

        // Start TCC
        s.sync();
        s.tc.count16_mut().ctrlbset.write(|w| w.cmd().retrigger());

        s
    }
}

)+}}

frequency_meter! {
    (TC3, Tcc2Tc3Clock, apbcmask, tc3_),
    (TC4, Tc4Tc5Clock, apbcmask, tc4_),
    (TC5, Tc4Tc5Clock, apbcmask, tc5_),
}
