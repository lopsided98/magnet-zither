#![no_std]
#![no_main]

mod command_parser;
mod console;
mod dual_slope_pwm;

use bsp::hal;
use bsp::pac;
use rtic::app;
use trinket_m0 as bsp;

#[app(device = bsp::pac, dispatchers = [EVSYS, DAC])]
mod app {
    use crate::bsp;
    use crate::command_parser;
    use crate::dual_slope_pwm;
    use crate::hal;
    use crate::pac;
    use core::fmt::Write;
    use hal::clock::GenericClockController;
    use hal::gpio::v2 as gpio;
    use hal::prelude::*;
    use hal::sercom::v2::uart;
    use panic_halt as _;

    use hal::rtc;

    macro_rules! uart_println {
        ($uart:expr, $($arg:tt)*) => {
            $uart.lock(|s| writeln!(s as &mut dyn embedded_hal::serial::Write<_, Error = _>, $($arg)*))
        };
    }

    #[shared]
    struct Shared {
        uart_tx: uart::Uart<uart::Config<bsp::UartPads>, uart::TxDuplex>,
    }

    #[local]
    struct Local {
        string: dual_slope_pwm::Pwm0,
        uart_rx: uart::Uart<uart::Config<bsp::UartPads>, uart::RxDuplex>,
        red_led: bsp::RedLed,
    }

    #[monotonic(binds = RTC, default = true)]
    type RtcMonotonic = rtc::Rtc<rtc::Count32Mode>;

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        let mut peripherals: pac::Peripherals = cx.device;

        let mut clocks = GenericClockController::with_internal_32kosc(
            peripherals.GCLK,
            &mut peripherals.PM,
            &mut peripherals.SYSCTRL,
            &mut peripherals.NVMCTRL,
        );
        let gclk0 = clocks.gclk0();
        let gclk1 = clocks.gclk1();

        let rtc_clock = clocks.rtc(&gclk1).unwrap();
        let rtc = rtc::Rtc::count32_mode(peripherals.RTC, rtc_clock.freq(), &mut peripherals.PM);

        let pins = bsp::Pins::new(peripherals.PORT);
        let red_led = pins.d13.into_push_pull_output();

        let (mut uart_rx, uart_tx) = bsp::uart(
            &mut clocks,
            115200.hz(),
            peripherals.SERCOM0,
            &mut peripherals.PM,
            pins.d3,
            pins.d4,
        )
        .split();
        uart_rx.enable_interrupts(uart::Flags::RXC);

        let string = dual_slope_pwm::Pwm0::new(
            &clocks.tcc0_tcc1(&gclk0).unwrap(),
            12.mhz(),
            peripherals.TCC0,
            &mut peripherals.PM,
        );

        let _string_pin: gpio::Pin<_, gpio::AlternateE> = pins.d0.into_mode();

        blink::spawn().unwrap();

        (
            Shared { uart_tx },
            Local {
                string,
                red_led,
                uart_rx,
            },
            init::Monotonics(rtc),
        )
    }

    #[task(
        shared = [uart_tx],
        local = [
            string,
            parser: command_parser::Parser = command_parser::Parser::new()
        ],
        capacity = 16
    )]
    fn handle_command(mut cx: handle_command::Context, data: u8) {
        let cmd = match cx.local.parser.parse(data) {
            Ok(Some(cmd)) => cmd,
            Err(e) => {
                uart_println!(cx.shared.uart_tx, "error: {0}", e).unwrap();
                return;
            }
            _ => return,
        };

        let string = cx.local.string;
        string.configure(cmd.period, cmd.amplitude);
    }

    #[task(binds = SERCOM0, local = [uart_rx], priority = 2)]
    fn uart_rx(cx: uart_rx::Context) {
        if let Ok(data) = cx.local.uart_rx.read() {
            handle_command::spawn(data).ok();
        }
    }

    #[task(local = [red_led])]
    fn blink(mut cx: blink::Context) {
        cx.local.red_led.toggle().unwrap();
        blink::spawn_after(rtc::Duration::millis(300)).ok();
    }
}
