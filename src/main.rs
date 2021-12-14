#![no_std]
#![no_main]

use bsp::hal;
use bsp::pac;
use itsybitsy_m0 as bsp;
use rtic::app;

mod const_assert;

mod dac;
mod pwm_dac;
mod string;

#[app(device = bsp::pac, dispatchers = [EVSYS, DAC])]
mod app {
    use embedded_midi::{MidiMessage, MidiParser};
    use hal::clock::GenericClockController;
    use hal::gpio::v2 as gpio;
    use hal::prelude::*;
    use hal::rtc;
    use hal::sercom::v2::uart;
    use hal::sercom::v2::Sercom0;
    use hal::typelevel::NoneT;
    use hal::usb::usb_device::bus::UsbBusAllocator;
    use hal::usb::usb_device::device::{UsbDevice, UsbDeviceBuilder, UsbVidPid};
    use hal::usb::UsbBus;
    use panic_halt as _;
    use seq_macro::seq;

    use string::{Controller, DacDriver};

    use crate::bsp;
    use crate::hal;
    use crate::pac;
    use crate::pwm_dac;
    use crate::string;

    // macro_rules! uart_println {
    //     ($uart:expr, $($arg:tt)*) => {
    //         $uart.lock(|s| writeln!(s as &mut dyn embedded_hal::serial::Write<_, Error = _>, $($arg)*))
    //     };
    // }

    const NUM_STRINGS: u8 = 8;

    macro_rules! for_each_string {
        ($($tts:tt)*) => { seq!(N in 0..8 { $($tts)* }); }
    }

    macro_rules! string_i_lock {
        ($cx:expr, $i:expr, $f:expr) => {
            for_each_string!(
                $cx.shared.strings.lock(|strings| match $i {
                    #(N => ($f)(&mut strings.N),)*
                    _ => panic!("String out of range")
                })
            )
        };
    }

    for_each_string!(
        pub struct DmaResources (
            #(string::dac_driver::DmaResources<u8>,)*
        );

        impl DmaResources {
            pub const fn new() -> Self {
                Self(
                    #(string::dac_driver::DmaResources::new(),)*
                )
            }
        }
    );

    pub struct Strings(
        string::ControllerImpl<DacDriver<pwm_dac::Channel<pac::TCC0, 0>>>,
        string::ControllerImpl<DacDriver<pwm_dac::Channel<pac::TCC0, 1>>>,
        string::ControllerImpl<DacDriver<pwm_dac::Channel<pac::TCC0, 2>>>,
        string::ControllerImpl<DacDriver<pwm_dac::Channel<pac::TCC0, 3>>>,
        string::ControllerImpl<DacDriver<pwm_dac::Channel<pac::TCC1, 0>>>,
        string::ControllerImpl<DacDriver<pwm_dac::Channel<pac::TCC1, 1>>>,
        string::ControllerImpl<DacDriver<pwm_dac::Channel<pac::TCC2, 0>>>,
        string::ControllerImpl<DacDriver<pwm_dac::Channel<pac::TCC2, 1>>>,
    );

    impl Strings {
        pub fn new(
            dac_tcc0: pwm_dac::PwmDac<pac::TCC0>,
            dac_tcc1: pwm_dac::PwmDac<pac::TCC1>,
            dac_tcc2: pwm_dac::PwmDac<pac::TCC2>,
            dma: &mut samd_dma::DMAController<samd_dma::storage::Storage8>,
            dma_resources: &'static mut DmaResources,
        ) -> Self {
            let dac_tcc0 = dac_tcc0.split();
            let dac_tcc1 = dac_tcc1.split();
            let dac_tcc2 = dac_tcc2.split();

            Self(
                string::ControllerImpl::new(
                    string::DacDriver::new(
                        dac_tcc0.0,
                        dma.take_channel::<samd_dma::consts::CH0>().unwrap(),
                        &mut dma_resources.0,
                    ),
                    string::Config {
                        period: 2527359.ns().into(),
                        attack_time: rtc::Duration::millis(200),
                        attack_amplitude: 255,
                        sustain_amplitude: 100,
                        release_time: rtc::Duration::millis(20),
                        release_amplitude: 0,
                    },
                ),
                string::ControllerImpl::new(
                    string::DacDriver::new(
                        dac_tcc0.1,
                        dma.take_channel::<samd_dma::consts::CH1>().unwrap(),
                        &mut dma_resources.1,
                    ),
                    string::Config {
                        period: 2251644.ns().into(),
                        attack_time: rtc::Duration::millis(100),
                        attack_amplitude: 255,
                        sustain_amplitude: 100,
                        release_time: rtc::Duration::millis(50),
                        release_amplitude: 100,
                    },
                ),
                string::ControllerImpl::new(
                    string::DacDriver::new(
                        dac_tcc0.2,
                        dma.take_channel::<samd_dma::consts::CH2>().unwrap(),
                        &mut dma_resources.2,
                    ),
                    string::Config {
                        period: 2024619.ns().into(),
                        attack_time: rtc::Duration::millis(200),
                        attack_amplitude: 255,
                        sustain_amplitude: 100,
                        release_time: rtc::Duration::millis(1000),
                        release_amplitude: 0,
                    },
                ),
                string::ControllerImpl::new(
                    string::DacDriver::new(
                        dac_tcc0.3,
                        dma.take_channel::<samd_dma::consts::CH3>().unwrap(),
                        &mut dma_resources.3,
                    ),
                    string::Config {
                        period: 1924965.ns().into(),
                        attack_time: rtc::Duration::millis(200),
                        attack_amplitude: 255,
                        sustain_amplitude: 100,
                        release_time: rtc::Duration::millis(1000),
                        release_amplitude: 0,
                    },
                ),
                string::ControllerImpl::new(
                    string::DacDriver::new(
                        dac_tcc1.0,
                        dma.take_channel::<samd_dma::consts::CH4>().unwrap(),
                        &mut dma_resources.4,
                    ),
                    string::Config {
                        period: 1696439.ns().into(),
                        attack_time: rtc::Duration::millis(200),
                        attack_amplitude: 255,
                        sustain_amplitude: 100,
                        release_time: rtc::Duration::millis(1000),
                        release_amplitude: 0,
                    },
                ),
                string::ControllerImpl::new(
                    string::DacDriver::new(
                        dac_tcc1.1,
                        dma.take_channel::<samd_dma::consts::CH5>().unwrap(),
                        &mut dma_resources.5,
                    ),
                    string::Config {
                        period: 1528888.ns().into(),
                        attack_time: rtc::Duration::millis(200),
                        attack_amplitude: 255,
                        sustain_amplitude: 100,
                        release_time: rtc::Duration::millis(1000),
                        release_amplitude: 0,
                    },
                ),
                string::ControllerImpl::new(
                    string::DacDriver::new(
                        dac_tcc2.0,
                        dma.take_channel::<samd_dma::consts::CH6>().unwrap(),
                        &mut dma_resources.6,
                    ),
                    string::Config {
                        period: 1437215.ns().into(),
                        attack_time: rtc::Duration::millis(200),
                        attack_amplitude: 255,
                        sustain_amplitude: 100,
                        release_time: rtc::Duration::millis(1000),
                        release_amplitude: 0,
                    },
                ),
                string::ControllerImpl::new(
                    string::DacDriver::new(
                        dac_tcc2.1,
                        dma.take_channel::<samd_dma::consts::CH7>().unwrap(),
                        &mut dma_resources.7,
                    ),
                    string::Config {
                        period: 1276699.ns().into(),
                        attack_time: rtc::Duration::millis(200),
                        attack_amplitude: 255,
                        sustain_amplitude: 100,
                        release_time: rtc::Duration::millis(1000),
                        release_amplitude: 0,
                    },
                ),
            )
        }
    }

    #[shared]
    struct Shared {
        strings: Strings,
    }

    #[local]
    struct Local {
        usb_device: UsbDevice<'static, UsbBus>,
        uart_rx: uart::Uart<uart::Config<uart::Pads<Sercom0, bsp::UartRx, NoneT>>, uart::Rx>,
        // red_led: bsp::RedLed,
        midi: MidiParser,
    }

    #[monotonic(binds = RTC, default = true)]
    type RtcMonotonic = rtc::Rtc<rtc::Count32Mode>;

    #[init(local = [
        usb_allocator: Option<UsbBusAllocator<UsbBus>> = None,
        dma_storage: samd_dma::storage::Storage8 = samd_dma::storage::Storage8::new(),
        dma_resources: DmaResources = DmaResources::new(),
    ])]
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

        *cx.local.usb_allocator = Some(bsp::usb_allocator(
            peripherals.USB,
            &mut clocks,
            &mut peripherals.PM,
            pins.usb_dm,
            pins.usb_dp,
        ));
        let usb_allocator = cx.local.usb_allocator.as_ref().unwrap();

        let usb_device = UsbDeviceBuilder::new(&usb_allocator, UsbVidPid(0x16c0, 0x27dd))
            .manufacturer("Ben Wolsieffer")
            .product("Magnet Zither")
            .serial_number("0000")
            .build();

        let mut dma = samd_dma::DMAController::init(peripherals.DMAC, cx.local.dma_storage);
        dma.enable();
        dma.enable_priority_level(samd_dma::Priority::Level0);

        let mut uart_rx = {
            let clock = &clocks.sercom0_core(&gclk0).unwrap();
            let pads = uart::Pads::default().rx(pins.d0);
            uart::Config::new(&mut peripherals.PM, peripherals.SERCOM0, pads, clock.freq())
                .baud(
                    115200.hz(),
                    uart::BaudMode::Fractional(uart::Oversampling::Bits16),
                )
                .enable()
        };
        uart_rx.enable_interrupts(uart::Flags::RXC);

        let tcc0_tcc1_clock = clocks.tcc0_tcc1(&gclk0).unwrap();
        let tcc2_tc3_clock = clocks.tcc2_tc3(&gclk0).unwrap();

        let dac_tcc0 = pwm_dac::PwmDac::<pac::TCC0>::new(
            &tcc0_tcc1_clock,
            25.khz(),
            peripherals.TCC0,
            &mut peripherals.PM,
        );
        let dac_tcc1 = pwm_dac::PwmDac::<pac::TCC1>::new(
            &tcc0_tcc1_clock,
            25.khz(),
            peripherals.TCC1,
            &mut peripherals.PM,
        );
        let dac_tcc2 = pwm_dac::PwmDac::<pac::TCC2>::new(
            &tcc2_tc3_clock,
            25.khz(),
            peripherals.TCC2,
            &mut peripherals.PM,
        );

        let _string_0_pin: gpio::Pin<_, gpio::AlternateE> = pins.d4.into_mode();
        let _string_1_pin: gpio::Pin<_, gpio::AlternateE> = pins.d3.into_mode();
        let _string_2_pin: gpio::Pin<_, gpio::AlternateF> = pins.d10.into_mode();
        let _string_3_pin: gpio::Pin<_, gpio::AlternateF> = pins.d12.into_mode();
        let _string_4_pin: gpio::Pin<_, gpio::AlternateE> = pins.d1.into_mode();
        let _string_5_pin: gpio::Pin<_, gpio::AlternateE> = pins.d9.into_mode();
        let _string_6_pin: gpio::Pin<_, gpio::AlternateE> = pins.d11.into_mode();
        let _string_7_pin: gpio::Pin<_, gpio::AlternateE> = pins.d13.into_mode();

        let strings = Strings::new(
            dac_tcc0,
            dac_tcc1,
            dac_tcc2,
            &mut dma,
            cx.local.dma_resources,
        );

        (
            Shared { strings },
            Local {
                usb_device,
                uart_rx,
                midi: MidiParser::new(),
            },
            init::Monotonics(rtc),
        )
    }

    #[task(
        shared = [strings],
        capacity = 8
    )]
    fn update_string(mut cx: update_string::Context, i: u8) {
        string_i_lock!(cx, i, |string: &mut string::ControllerImpl<
            DacDriver<_>,
        >| {
            if let Some(t) = string.update() {
                update_string::spawn_at(t, i).ok();
            }
        });
    }

    fn msg_to_note(msg: &MidiMessage) -> Option<embedded_midi::Note> {
        match msg {
            MidiMessage::NoteOn(_, note, _) => Some(*note),
            MidiMessage::NoteOff(_, note, _) => Some(*note),
            _ => None,
        }
    }

    fn note_to_string(note: embedded_midi::Note) -> Option<u8> {
        match note.into() {
            67 => Some(0),
            69 => Some(1),
            71 => Some(2),
            72 => Some(3),
            74 => Some(4),
            76 => Some(5),
            77 => Some(6),
            79 => Some(7),
            _ => None,
        }
    }

    #[task(
        shared = [strings],
        local = [midi],
        capacity = 16
    )]
    fn handle_midi(mut cx: handle_midi::Context, data: u8) {
        let msg: MidiMessage = if let Some(msg) = cx.local.midi.parse_byte(data) {
            msg
        } else {
            return;
        };

        if let Some(i) = msg_to_note(&msg).and_then(note_to_string) {
            string_i_lock!(cx, i, |string: &mut string::ControllerImpl<
                DacDriver<_>,
            >| {
                if let Some(t) = match msg {
                    MidiMessage::NoteOn(_, _, velocity) => string.on(velocity.into()),
                    MidiMessage::NoteOff(_, _, _velocity) => string.off(127),
                    _ => None,
                } {
                    update_string::spawn_at(t, i).ok();
                }
            });
        }
    }

    #[task(
        shared = [strings],
        capacity = 8
    )]
    fn fill_buffer(
        mut cx: fill_buffer::Context,
        string: u8,
        buffer: string::dac_driver::FillableBuffer<u8>,
    ) {
        let buffer = buffer.fill();

        string_i_lock!(cx, string, |string: &mut string::ControllerImpl<
            DacDriver<_>,
        >| {
            // https://github.com/rust-lang/rust/issues/42574
            let buffer = buffer;
            string.driver_mut().submit(buffer)
        });
    }

    #[task(
        binds = DMAC,
        shared = [strings],
        priority = 3
    )]
    fn dmac_interrupt(mut cx: dmac_interrupt::Context) {
        for i in 0..NUM_STRINGS {
            let mut buffer = None;

            string_i_lock!(cx, i, |string: &mut string::ControllerImpl<
                DacDriver<_>,
            >| buffer = string.driver_mut().request());
            if let Some(buffer) = buffer {
                fill_buffer::spawn(i, buffer).ok();
            }
        }
    }

    #[task(binds = SERCOM0, local = [uart_rx], priority = 2)]
    fn uart_rx(cx: uart_rx::Context) {
        let rx = cx.local.uart_rx;
        if let Ok(data) = rx.read() {
            handle_midi::spawn(data).ok();
        }
        rx.clear_status(uart::Status::all());
        rx.clear_flags(uart::Flags::RXC);
    }

    #[task(binds = USB, local = [usb_device], priority = 2)]
    fn usb_interrupt(cx: usb_interrupt::Context) {
        let usb_dev: &mut UsbDevice<UsbBus> = cx.local.usb_device;
        usb_dev.poll(&mut []);
    }
}
