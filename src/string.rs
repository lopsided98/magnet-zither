use crate::app::monotonics;
use crate::hal::rtc::{Duration, Instant};
use crate::hal::time::Nanoseconds;

pub mod dac_driver;
mod led_driver;

pub use dac_driver::DacDriver;
pub use led_driver::LedDriver;

pub trait Driver {
    fn set(&mut self, period: Nanoseconds, amplitude: u8, invert: bool);
}

enum State {
    Attack { velocity: u8 },
    Sustain { velocity: u8 },
    Release { velocity: u8 },
    Off,
}

pub struct Config {
    pub period: Nanoseconds,
    pub attack_time: Duration,
    pub attack_amplitude: u8,
    pub sustain_amplitude: u8,
    pub release_time: Duration,
    pub release_amplitude: u8,
}

pub trait Controller {
    fn on(&mut self, velocity: u8) -> Option<Instant>;

    fn off(&mut self, velocity: u8) -> Option<Instant>;

    fn update(&mut self) -> Option<Instant>;
}

pub struct ControllerImpl<D: Driver> {
    driver: D,
    config: Config,
    state: State,
}

impl<D: Driver> ControllerImpl<D> {
    const MAX_VELOCITY: u8 = 127;

    pub fn new(driver: D, config: Config) -> Self {
        monotonics::now();

        Self {
            driver,
            config,
            state: State::Off,
        }
    }

    fn apply_velocity(amplitude: u8, velocity: u8) -> u8 {
        (amplitude as u32 * velocity.min(Self::MAX_VELOCITY) as u32 / Self::MAX_VELOCITY as u32)
            as u8
    }

    fn update_driver(&mut self) {
        let mut invert = false;
        let amplitude = match self.state {
            State::Off => 0,
            State::Attack { velocity } => {
                Self::apply_velocity(self.config.attack_amplitude, velocity)
            }
            State::Sustain { velocity } => {
                Self::apply_velocity(self.config.sustain_amplitude, velocity)
            }
            State::Release { velocity } => {
                invert = true;
                Self::apply_velocity(self.config.release_amplitude, velocity)
            }
        };
        self.driver.set(self.config.period, amplitude, invert);
    }

    pub fn driver_mut(&mut self) -> &mut D {
        &mut self.driver
    }
}

impl<D: Driver> Controller for ControllerImpl<D> {
    fn on(&mut self, velocity: u8) -> Option<Instant> {
        if let Some((state, duration)) = match &self.state {
            State::Off | State::Release { .. } => {
                Some((State::Attack { velocity }, self.config.attack_time))
            }
            _ => None,
        } {
            self.state = state;
            self.update_driver();
            Some(monotonics::now() + duration)
        } else {
            None
        }
    }

    fn off(&mut self, _velocity: u8) -> Option<Instant> {
        if let Some((state, duration)) = match &self.state {
            State::Attack { velocity } | State::Sustain { velocity } => {
                Some((State::Release { velocity: *velocity }, self.config.release_time))
            }
            _ => None,
        } {
            self.state = state;
            self.update_driver();
            Some(monotonics::now() + duration)
        } else {
            None
        }
    }

    fn update(&mut self) -> Option<Instant> {
        let (state, duration) = match &self.state {
            State::Attack { velocity } => (
                Some(State::Sustain {
                    velocity: *velocity,
                }),
                None,
            ),
            State::Release { .. } => (Some(State::Off), None),
            _ => (None, None),
        };

        if let Some(state) = state {
            self.state = state;
            self.update_driver();
        }

        duration.map(|d| monotonics::now() + d)
    }
}
