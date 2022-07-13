use hal::rtc::{Duration, Instant};
use hal::time::{Nanoseconds, U32Ext};
use num_rational::Ratio;

pub use dac_driver::DacDriver;

use crate::ac;
use crate::app::monotonics;
use crate::hal;
use crate::pac;

pub mod dac_driver;

pub trait Driver {
    fn set(&mut self, period: Nanoseconds, amplitude: u8, invert: bool);
}

struct ScheduledState {
    end: Option<Instant>,
    state: State,
}

enum State {
    Attack { velocity: u8, harmonic: u8 },
    Sustain { velocity: u8, harmonic: u8 },
    Release { velocity: u8, harmonic: u8 },
    WaitStabilize,
    SampleFrequency,
    Off,
}

impl State {
    pub const fn schedule(self, end: Instant) -> ScheduledState {
        ScheduledState {
            end: Some(end),
            state: self,
        }
    }

    pub const fn indefinite(self) -> ScheduledState {
        ScheduledState {
            end: None,
            state: self,
        }
    }
}

pub struct Config {
    pub period: Nanoseconds,
    pub attack_time: Duration,
    pub attack_amplitude: u8,
    pub sustain_amplitude: u8,
    pub release_time: Duration,
    pub release_amplitude: u8,
    pub stabilize_time: Duration,
    pub sample_time: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            period: Nanoseconds::from(500.hz()),
            attack_time: Duration::millis(100),
            attack_amplitude: 255,
            sustain_amplitude: 100,
            release_time: Duration::millis(0),
            release_amplitude: 0,
            stabilize_time: Duration::millis(50),
            sample_time: Duration::millis(500),
        }
    }
}

pub struct Controller<D: Driver> {
    driver: D,
    freq_meter: Option<ac::FrequencyMeter<pac::TC3>>,
    config: Config,
    state: ScheduledState,
}

impl<D: Driver> Controller<D> {
    const MAX_VELOCITY: u8 = 127;

    pub fn new(
        driver: D,
        freq_meter: Option<ac::FrequencyMeter<pac::TC3>>,
        config: Config,
    ) -> Self {
        monotonics::now();

        Self {
            driver,
            freq_meter,
            config,
            state: State::Off.indefinite(),
        }
    }

    fn apply_velocity(amplitude: u8, velocity: u8) -> u8 {
        (amplitude as u32 * velocity.min(Self::MAX_VELOCITY) as u32 / Self::MAX_VELOCITY as u32)
            as u8
    }

    fn update_driver(&mut self) {
        let mut invert = false;
        let (amplitude, harmonic) = match self.state.state {
            State::Attack { velocity, harmonic } => (
                Self::apply_velocity(self.config.attack_amplitude, velocity),
                harmonic,
            ),
            State::Sustain { velocity, harmonic } => (
                Self::apply_velocity(self.config.sustain_amplitude, velocity),
                harmonic,
            ),
            State::Release { velocity, harmonic } => {
                invert = true;
                (
                    Self::apply_velocity(self.config.release_amplitude, velocity),
                    harmonic,
                )
            }
            _ => (0, 1),
        };
        self.driver.set(
            (self.config.period.0 / harmonic as u32).ns(),
            amplitude,
            invert,
        );

        if let Some(freq_meter) = &self.freq_meter {
            match self.state.state {
                State::SampleFrequency => freq_meter.enable_interrupts(),
                _ => freq_meter.disable_interrupts(),
            }
        }
    }

    pub fn driver_mut(&mut self) -> &mut D {
        &mut self.driver
    }

    pub fn on(&mut self, velocity: u8, harmonic: u8) -> Option<Instant> {
        let now = monotonics::now();

        match &self.state.state {
            State::Off | State::Release { .. } | State::WaitStabilize | State::SampleFrequency => {
                Some(State::Attack { velocity, harmonic }.schedule(now + self.config.attack_time))
            }
            _ => None,
        }
        .map(|state| {
            self.state = state;
            self.update_driver();
        })
        .and(self.state.end)
    }

    pub fn off(&mut self, _velocity: u8) -> Option<Instant> {
        let now = monotonics::now();

        match &self.state.state {
            State::Attack { velocity, harmonic } | State::Sustain { velocity, harmonic } => Some(
                State::Release {
                    velocity: *velocity,
                    harmonic: *harmonic,
                }
                .schedule(now + self.config.release_time),
            ),
            _ => None,
        }
        .map(|state| {
            self.state = state;
            self.update_driver();
        })
        .and(self.state.end)
    }

    pub fn update(&mut self) -> Option<Instant> {
        let now = monotonics::now();

        // When new commands come in, old updates remain scheduled but are no longer valid.
        // Therefore, we need to check whether the current state is really supposed to end
        // now.
        if let Some(end) = self.state.end {
            if now < end {
                return None;
            }
        }

        let start = self.state.end.unwrap_or(now);

        match &self.state.state {
            State::Attack { velocity, harmonic } => Some(
                State::Sustain {
                    velocity: *velocity,
                    harmonic: *harmonic,
                }
                .indefinite(),
            ),
            State::Release { .. } if self.freq_meter.is_some() => {
                Some(State::WaitStabilize.schedule(start + self.config.stabilize_time))
            }
            State::Release { .. } => Some(State::Off.indefinite()),
            State::WaitStabilize => {
                Some(State::SampleFrequency.schedule(start + self.config.sample_time))
            }
            State::SampleFrequency => Some(State::Off.indefinite()),
            State::Sustain { .. } | State::Off => None,
        }
        .map(|state| {
            self.state = state;
            self.update_driver();
        })
        .and(self.state.end)
    }

    pub fn sample_frequency(&mut self) {
        // Only sample frequency when in the relevant state
        if let State::SampleFrequency = self.state.state {
        } else {
            return;
        }

        if let Some(freq_meter) = &self.freq_meter {
            // Ignore readings if the error flag is set
            if freq_meter.on_interrupt().is_err() {
                // writeln!(uart as &mut dyn hal::ehal::serial::Write<_, Error = _>, "error");
                return;
            }

            // Ignore outliers more than 10% different from the current value
            let period_sample = freq_meter.period_ns();
            if ((self.config.period.0 as i32 - period_sample.0 as i32).abs() as u32)
                > self.config.period.0 / 10
            {
                // writeln!(uart as &mut dyn hal::ehal::serial::Write<_, Error = _>, "out of range: {}", Hertz::from(period_sample).0);
                return;
            }

            self.config.period = (Ratio::from_integer(self.config.period.0) * Ratio::new(9, 10)
                + Ratio::from_integer(period_sample.0) * Ratio::new(1, 10))
            .round()
            .numer()
            .ns();

            // writeln!(uart as &mut dyn hal::ehal::serial::Write<_, Error = _>, "sample: {}, new: {}", Hertz::from(period_sample).0, Hertz::from(self.config.period).0);
        }
    }
}
