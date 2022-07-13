#![no_std]
#![no_main]

use bsp::hal;
use bsp::pac;
use itsybitsy_m0 as bsp;
use rtic::app;

mod const_assert;

mod ac;
mod dac;
mod evsys;
mod pwm_dac;
mod string;

#[app(device = bsp::pac, dispatchers = [EVSYS, DAC])]
mod app {
    use hal::clock::GenericClockController;
    use hal::gpio::v2 as gpio;
    use hal::prelude::*;
    use hal::rtc;
    use hal::usb::usb_device::bus::UsbBusAllocator;
    use hal::usb::usb_device::device::{UsbDevice, UsbDeviceBuilder, UsbVidPid};
    use hal::usb::UsbBus;
    use panic_halt as _;
    use seq_macro::seq;
    use usbd_midi::data::midi;
    use usbd_midi::data::usb_midi::midi_packet_reader::MidiPacketBufferReader;

    use crate::ac;
    use crate::bsp;
    use crate::evsys;
    use crate::hal;
    use crate::pac;
    use crate::pwm_dac;
    use crate::string;

// macro_rules! uart_println {
    //     ($uart:expr, $($arg:tt)*) => {{
    //         use core::fmt::Write;
    //         $uart.lock(|s| writeln!(s as &mut dyn $crate::hal::ehal::serial::Write<_, Error = _>, $($arg)*))
    //     }}
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
        string::Controller<string::DacDriver<pwm_dac::Channel<pac::TCC0, 0>>>,
        string::Controller<string::DacDriver<pwm_dac::Channel<pac::TCC0, 1>>>,
        string::Controller<string::DacDriver<pwm_dac::Channel<pac::TCC0, 2>>>,
        string::Controller<string::DacDriver<pwm_dac::Channel<pac::TCC0, 3>>>,
        string::Controller<string::DacDriver<pwm_dac::Channel<pac::TCC1, 0>>>,
        string::Controller<string::DacDriver<pwm_dac::Channel<pac::TCC1, 1>>>,
        string::Controller<string::DacDriver<pwm_dac::Channel<pac::TCC2, 0>>>,
        string::Controller<string::DacDriver<pwm_dac::Channel<pac::TCC2, 1>>>,
    );

    impl Strings {
        pub fn new(
            dac_tcc0: pwm_dac::PwmDac<pac::TCC0>,
            dac_tcc1: pwm_dac::PwmDac<pac::TCC1>,
            dac_tcc2: pwm_dac::PwmDac<pac::TCC2>,
            _freq_meter: ac::FrequencyMeter<pac::TC3>,
            dma: &mut samd_dma::DMAController<samd_dma::storage::Storage8>,
            dma_resources: &'static mut DmaResources,
        ) -> Self {
            let dac_tcc0 = dac_tcc0.split();
            let dac_tcc1 = dac_tcc1.split();
            let dac_tcc2 = dac_tcc2.split();

            Self(
                string::Controller::new(
                    string::DacDriver::new(
                        dac_tcc0.0,
                        dma.take_channel::<samd_dma::consts::CH0>().unwrap(),
                        &mut dma_resources.0,
                    ),
                    None,
                    string::Config {
                        period: 2527359.ns().into(),
                        attack_time: rtc::Duration::millis(200),
                        sustain_amplitude: 100,
                        release_amplitude: 0,
                        ..string::Config::default()
                    },
                ),
                string::Controller::new(
                    string::DacDriver::new(
                        dac_tcc0.1,
                        dma.take_channel::<samd_dma::consts::CH1>().unwrap(),
                        &mut dma_resources.1,
                    ),
                    None,
                    string::Config {
                        period: 2251644.ns().into(),
                        attack_time: rtc::Duration::millis(100),
                        sustain_amplitude: 255,
                        release_amplitude: 100,
                        ..string::Config::default()
                    },
                ),
                string::Controller::new(
                    string::DacDriver::new(
                        dac_tcc0.2,
                        dma.take_channel::<samd_dma::consts::CH2>().unwrap(),
                        &mut dma_resources.2,
                    ),
                    None,
                    string::Config {
                        period: 2024619.ns().into(),
                        attack_time: rtc::Duration::millis(200),
                        sustain_amplitude: 100,
                        release_amplitude: 0,
                        ..string::Config::default()
                    },
                ),
                string::Controller::new(
                    string::DacDriver::new(
                        dac_tcc0.3,
                        dma.take_channel::<samd_dma::consts::CH3>().unwrap(),
                        &mut dma_resources.3,
                    ),
                    None,
                    string::Config {
                        period: 1924965.ns().into(),
                        attack_time: rtc::Duration::millis(200),
                        sustain_amplitude: 100,
                        release_amplitude: 0,
                        ..string::Config::default()
                    },
                ),
                string::Controller::new(
                    string::DacDriver::new(
                        dac_tcc1.0,
                        dma.take_channel::<samd_dma::consts::CH4>().unwrap(),
                        &mut dma_resources.4,
                    ),
                    None,
                    string::Config {
                        period: 1696439.ns().into(),
                        attack_time: rtc::Duration::millis(200),
                        sustain_amplitude: 100,
                        release_amplitude: 0,
                        ..string::Config::default()
                    },
                ),
                string::Controller::new(
                    string::DacDriver::new(
                        dac_tcc1.1,
                        dma.take_channel::<samd_dma::consts::CH5>().unwrap(),
                        &mut dma_resources.5,
                    ),
                    None,
                    string::Config {
                        period: 1528888.ns().into(),
                        attack_time: rtc::Duration::millis(200),
                        attack_amplitude: 255,
                        sustain_amplitude: 100,
                        release_amplitude: 0,
                        ..string::Config::default()
                    },
                ),
                string::Controller::new(
                    string::DacDriver::new(
                        dac_tcc2.0,
                        dma.take_channel::<samd_dma::consts::CH6>().unwrap(),
                        &mut dma_resources.6,
                    ),
                    None,
                    string::Config {
                        period: 1442793.ns().into(),
                        attack_time: rtc::Duration::millis(200),
                        attack_amplitude: 255,
                        sustain_amplitude: 100,
                        release_amplitude: 0,
                        ..string::Config::default()
                    },
                ),
                string::Controller::new(
                    string::DacDriver::new(
                        dac_tcc2.1,
                        dma.take_channel::<samd_dma::consts::CH7>().unwrap(),
                        &mut dma_resources.7,
                    ),
                    None,
                    string::Config {
                        period: 1276699.ns().into(),
                        attack_time: rtc::Duration::millis(200),
                        attack_amplitude: 255,
                        sustain_amplitude: 100,
                        release_amplitude: 0,
                        ..string::Config::default()
                    },
                ),
            )
        }
    }

    #[shared]
    struct Shared {
        // uart_tx: uart::Uart<uart::Config<uart::Pads<Sercom0, NoneT, bsp::UartTx>>, uart::Tx>,
        strings: Strings,
    }

    #[local]
    struct Local {
        usb_device: UsbDevice<'static, UsbBus>,
        usb_midi: usbd_midi::midi_device::MidiClass<'static, UsbBus>,
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

        // let uart_tx = {
        //     let clock = &clocks.sercom0_core(&gclk0).unwrap();
        //     let pads = uart::Pads::default().tx(pins.d1);
        //     uart::Config::new(&peripherals.PM, peripherals.SERCOM0, pads, clock.freq())
        //         .baud(
        //             115200.hz(),
        //             uart::BaudMode::Fractional(uart::Oversampling::Bits16),
        //         )
        //         .enable()
        // };

        *cx.local.usb_allocator = Some(bsp::usb_allocator(
            peripherals.USB,
            &mut clocks,
            &mut peripherals.PM,
            pins.usb_dm,
            pins.usb_dp,
        ));
        let usb_allocator = cx.local.usb_allocator.as_ref().unwrap();

        let usb_midi = usbd_midi::midi_device::MidiClass::new(&usb_allocator, 0, 1).unwrap();

        let usb_device = UsbDeviceBuilder::new(&usb_allocator, UsbVidPid(0x16c0, 0x5e4))
            .manufacturer("Ben Wolsieffer")
            .product("Magnet Zither")
            .serial_number("0000")
            .device_class(usbd_midi::data::usb::constants::USB_AUDIO_CLASS)
            .device_sub_class(usbd_midi::data::usb::constants::USB_MIDISTREAMING_SUBCLASS)
            .build();

        let mut dma = samd_dma::DMAController::init(peripherals.DMAC, cx.local.dma_storage);
        dma.enable();
        dma.enable_priority_level(samd_dma::Priority::Level0);

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

        let _ac_pos_pin: gpio::Pin<_, gpio::AlternateB> = pins.a3.into_mode();
        let _ac_neg_pin: gpio::Pin<_, gpio::AlternateB> = pins.a4.into_mode();
        let _ac_comp_pin: gpio::Pin<_, gpio::AlternateH> = pins.miso.into_mode();

        let evsys = evsys::EventSystem::new(peripherals.EVSYS, &peripherals.PM).split();

        let _ac = ac::AnalogComparator::new(
            clocks.ac_ana(&gclk0).unwrap(),
            clocks.ac_dig(&gclk0).unwrap(),
            peripherals.AC,
            &peripherals.PM,
        );

        let freq =
            ac::FrequencyMeter::<pac::TC3>::new(&tcc2_tc3_clock, peripherals.TC3, &peripherals.PM);

        let evsys_ac_channel = evsys.0;
        evsys_ac_channel.user(evsys::User::Tc3);
        evsys_ac_channel.config(evsys::Path::ASYNCHRONOUS, evsys::EventGenerator::AcComp0);

        let strings = Strings::new(
            dac_tcc0,
            dac_tcc1,
            dac_tcc2,
            freq,
            &mut dma,
            cx.local.dma_resources,
        );

        (
            Shared { strings },
            Local {
                usb_device,
                usb_midi,
            },
            init::Monotonics(rtc),
        )
    }

    #[task(
        shared = [strings],
        capacity = 8
    )]
    fn update_string(mut cx: update_string::Context, i: u8) {
        string_i_lock!(cx, i, |string: &mut string::Controller<_>| {
            if let Some(t) = string.update() {
                update_string::spawn_at(t, i).ok();
            }
        });
    }

    fn msg_to_note(msg: &midi::message::Message) -> Option<midi::notes::Note> {
        match msg {
            midi::message::Message::NoteOn(_, note, _) => Some(*note),
            midi::message::Message::NoteOff(_, note, _) => Some(*note),
            _ => None,
        }
    }

    fn note_to_string(note: midi::notes::Note) -> Option<u8> {
        use midi::notes::Note::*;
        match note.into() {
            G4 => Some(0),
            A4 => Some(1),
            B4 => Some(2),
            C5 => Some(3),
            D5 => Some(4),
            E5 => Some(5),
            F5 => Some(6),
            G5 => Some(7),
            _ => None,
        }
    }

    #[task(
        shared = [strings],
        capacity = 16
    )]
    fn handle_midi(mut cx: handle_midi::Context, msg: midi::message::Message) {
        if let Some(i) = msg_to_note(&msg).and_then(note_to_string) {
            string_i_lock!(cx, i, |string: &mut string::Controller<_>| {
                if let Some(t) = match msg {
                    midi::message::Message::NoteOn(_, _, velocity) => string.on(velocity.into()),
                    midi::message::Message::NoteOff(_, _, _velocity) => string.off(127),
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

        string_i_lock!(cx, string, |string: &mut string::Controller<
            string::DacDriver<_>,
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

            string_i_lock!(cx, i, |string: &mut string::Controller<
                string::DacDriver<_>,
            >| buffer = string.driver_mut().request());
            if let Some(buffer) = buffer {
                fill_buffer::spawn(i, buffer).ok();
            }
        }
    }

    #[task(binds = TC3, shared = [strings])]
    fn freq_interrupt(mut cx: freq_interrupt::Context) {
        cx.shared
            .strings
            .lock(|strings| strings.1.sample_frequency());
    }

    #[task(binds = USB, local = [usb_device, usb_midi], priority = 2)]
    fn usb_interrupt(cx: usb_interrupt::Context) {
        let usb_device: &mut UsbDevice<UsbBus> = cx.local.usb_device;
        let usb_midi: &mut usbd_midi::midi_device::MidiClass<_> = cx.local.usb_midi;

        if !usb_device.poll(&mut [usb_midi]) {
            return;
        }

        let mut buffer = [0; 64];
        if let Ok(size) = usb_midi.read(&mut buffer) {
            let buffer_reader = MidiPacketBufferReader::new(&buffer, size);
            for packet in buffer_reader.into_iter() {
                if let Ok(packet) = packet {
                    handle_midi::spawn(packet.message).ok();
                }
            }
        }
    }
}
