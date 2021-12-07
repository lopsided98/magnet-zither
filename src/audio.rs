use atsamd_hal::gpio::v2 as gpio;
use atsamd_hal::hal::timer::CountDown;
use hal::pac;
use hal::prelude::*;
use hal::time::U32Ext;

use super::hal;
use core::future::Future;
use core::task::{Context, Poll, Waker};
use core::pin::Pin;

struct Dac {
    dac: pac::DAC,
    waker: Option<Waker>
}

impl Dac {
    pub fn new(
        pm: &mut pac::PM,
        _: hal::clock::DacClock,
        dac: pac::DAC,
        _: gpio::Pa2<gpio::AlternateB>,
    ) -> Self {
        // Enable APB clock
        pm.apbcmask.modify(|_, w| w.dac_().set_bit());

        // Left adjust data, allows a 16-bit value to be written to the data
        // register and the 10 high bits will be used.
        dac.ctrlb.write(|w| w.leftadj().set_bit().refsel().int1v());

        // Enable sync interrupt
        dac.intenset.write(|w| w.syncrdy().set_bit());

        Self { dac, waker: None }
    }

    pub fn enable_output(&mut self, enable: bool) {
        self.dac.ctrlb.modify(|_, w| w.eoen().bit(enable));
    }

    pub fn enable_start_event_input(&mut self, enable: bool) {
        self.dac.evctrl.modify(|_, w| w.startei().bit(enable));
    }

    pub async fn set_value(&mut self, value: u16) {
        self.dac.data.write(|w| unsafe { w.bits(value) });
        self.wait_for_sync().await
    }

    pub async fn enable(&mut self, enable: bool) {
        self.dac.ctrla.modify(|_, w| w.enable().bit(enable));
        self.wait_for_sync().await
    }

    fn is_syncing(&self) -> bool {
        self.dac.status.read().syncbusy().bit_is_set()
    }

    fn wait_for_sync(&mut self) -> impl Future<Output=()> + '_ {
        struct WaitSyncFuture<'a> {
            dac: &'a mut Dac
        }

        impl<'a> Future for WaitSyncFuture<'a> {
            type Output = ();

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                if self.dac.is_syncing() {
                    self.dac.waker = Some(cx.waker().clone());
                    Poll::Pending
                } else {
                    Poll::Ready(())
                }
            }
        }

        WaitSyncFuture { dac: self }
    }

    pub fn interrupt_handler(&mut self){
        self.waker.take().map(|w| w.wake());
    }
}

#[derive(PartialEq)]
enum State {
    WaitBuffer,
    Play,
}

pub struct AudioDac<TC: hal::timer::Count16> {
    dac: Dac,
    dma_channel: samd_dma::Channel,
    timer: hal::timer::TimerCounter<TC>,
    buffer: Option<&'static mut [u16]>,
    state: State,
}

impl<TC: hal::timer::Count16> AudioDac<TC> {
    pub fn new(
        pm: &mut pac::PM,
        dac_clock: hal::clock::DacClock,
        dac: pac::DAC,
        output: gpio::Pa2<gpio::PfB>,
        mut dma_channel: samd_dma::Channel,
        mut timer: hal::timer::TimerCounter<TC>,
        _evsys_clock: hal::clock::Evsys0Clock,
        // FIXME: create wrapper
        evsys: &mut pac::EVSYS,
        event_channel: u8,
    ) -> Self {
        // Configure DMA channel
        // Only transfer one sample each time we are triggered
        dma_channel.set_trigger_action(samd_dma::TriggerAction::Beat);
        // Trigger on DAC databuf empty
        dma_channel.set_source(samd_dma::TriggerSource::DacEmpty);
        dma_channel.enable_interrupts(samd_dma::Interrupts::SUSP);
        let first_descriptor = dma_channel.get_first_descriptor();
        // Link descriptor to itself
        let first_descriptor_ptr: *mut samd_dma::TransferDescriptor = first_descriptor;
        first_descriptor.link_descriptor(first_descriptor_ptr);
        first_descriptor.set_beat_size(samd_dma::BeatSize::HWord);
        first_descriptor.set_step_size(samd_dma::StepSize::X1);
        first_descriptor.set_step_selection(true);
        first_descriptor.set_src_addr_increment(true);
        first_descriptor.set_dst_addr(dac.databuf.as_ptr() as *const ());
        first_descriptor.set_dest_addr_increment(false);
        // Suspend and wait for next buffer after transfer completes
        first_descriptor.set_block_action(samd_dma::BlockAction::Suspend);
        // Don't have a buffer yet, so mark descriptor as invalid. This will
        // immediately suspend the transfer when it is triggered.
        first_descriptor.set_invalid();

        let mut dac = Dac::new(pm, dac_clock, dac, output);

        // Enable EVSYS clock
        pm.apbcmask.modify(|_, w| w.evsys_().set_bit());
        // Connect event output to DAC
        evsys.user.write(|w| {
            unsafe {
                w.channel()
                    .bits(event_channel + 1)
                    .user()
                    // FIXME: use enum
                    .bits(0x1B) // DAC START
            }
        });
        // Configure timer overflow event
        evsys.channel.write(|w| {
            unsafe { w.channel().bits(event_channel) }
                .path()
                .asynchronous();
            // FIXME: don't hardcode
            unsafe { w.evgen().bits(0x33) } // TC3 OVF
        });
        dac.enable_start_event_input(true);

        Self {
            dac,
            dma_channel,
            timer,
            buffer: None,
            state: State::WaitBuffer,
        }
    }

    pub async fn enable(&mut self) {
        self.dac.enable(true);
        self.dac.enable_output(true);
        self.dma_channel.enable();
        self.timer.start(44100.hz());
    }

    pub fn update(&mut self) -> bool {
        let dma_flags = self.dma_channel.get_interrupt_flags();
        self.dma_channel.clear_interrupt_flags();

        // When we get a suspend interrupt, the DMA transfer finished and we
        // are waiting for another buffer. User code must supply a new buffer
        // before the next sample time
        if dma_flags.intersects(samd_dma::Interrupts::SUSP) {
            self.state = State::WaitBuffer;
        }

        self.state == State::WaitBuffer
    }

    pub fn take_buffer(&mut self) -> Option<&'static mut [u16]> {
        self.dma_channel.get_first_descriptor().set_invalid();
        core::mem::replace(&mut self.buffer, None)
    }

    pub fn swap_buffers(&mut self, buf: &'static mut [u16]) -> Option<&'static mut [u16]> {
        let descriptor = self.dma_channel.get_first_descriptor();
        descriptor.set_invalid();
        // src address needs to point to end of buffer
        descriptor.set_src_addr(unsafe { buf.as_mut_ptr().add(buf.len()) } as *const ());
        descriptor.set_block_count(buf.len() as u16);
        descriptor.set_valid();

        if let State::WaitBuffer = self.state {
            self.state = State::Play;
            self.dma_channel.resume();
        }
        core::mem::replace(&mut self.buffer, Some(buf))
    }

    pub fn dac_interrupt_handler(&mut self) {
        self.dac.interrupt_handler();
    }
}
