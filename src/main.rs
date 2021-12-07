#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use bsp::hal;
use bsp::pac;
use embassy::executor::Spawner;
use embassy::time::{Duration, Timer};
use hal::clock::GenericClockController;
use hal::gpio::v2 as gpio;
use hal::prelude::*;
use panic_halt as _;
use trinket_m0 as bsp;

mod dual_slope_pwm;

#[embassy::main]
async fn main(_spawner: Spawner, mut peripherals: pac::Peripherals) {
    let mut clocks = GenericClockController::with_internal_32kosc(
        peripherals.GCLK,
        &mut peripherals.PM,
        &mut peripherals.SYSCTRL,
        &mut peripherals.NVMCTRL,
    );
    let gclk0 = clocks.gclk0();
    let gclk1 = clocks.gclk1();

    // Setup RTC for embassy
    // This should really be done in embassy_atsamd but we can't use the atsamd_hal drivers if we
    // do that.
    clocks.rtc(&gclk1).unwrap();
    embassy_atsamd::time_driver::init(embassy_atsamd::interrupt::Priority::P0);

    let mut pins = bsp::Pins::new(peripherals.PORT);
    let mut red_led = pins.d13.into_push_pull_output();

    let uart = bsp::uart(
        &mut clocks,
        115200.hz(),
        peripherals.SERCOM0,
        &mut peripherals.PM,
        pins.d3,
        pins.d4,
    );

    let string = dual_slope_pwm::Pwm0::new(
        &clocks.tcc0_tcc1(&gclk0).unwrap(),
        12.mhz(),
        peripherals.TCC0,
        &mut peripherals.PM,
    );

    let string_pin: gpio::Pin<_, gpio::AlternateB> = pins.d0.into_mode();

    red_led.set_high().unwrap();
    loop {
        red_led.set_high().unwrap();
        Timer::after(Duration::from_millis(300)).await;
        red_led.set_low().unwrap();
        Timer::after(Duration::from_millis(300)).await;
    }
}
