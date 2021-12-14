use super::Driver;
use crate::hal::time::Nanoseconds;
use embedded_hal::digital::v2 as digital;

pub struct LedDriver<P: digital::OutputPin> {
    pin: P,
}

impl<P: digital::OutputPin> LedDriver<P> {
    pub fn new(pin: P) -> Self {
        Self { pin }
    }
}

impl<P: digital::OutputPin> Driver for LedDriver<P> {
    fn set(&mut self, _period: Nanoseconds, amplitude: u8, _invert: bool) {
        self.pin
            .set_state(if amplitude > 0 {
                digital::PinState::High
            } else {
                digital::PinState::Low
            })
            .ok();
    }
}
