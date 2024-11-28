// SPDX-License-Identifier: GPL-3.0-or-later
use num_traits::{cast, PrimInt, Zero};

use crate::dac::Dac;
use crate::hal::time::{Nanoseconds, U32Ext};
use crate::string::Driver;

pub const BUFFER_SIZE: usize = 512;

// Must be Copy to be used in an array
#[derive(Copy, Clone)]
pub struct DmaResources<S> {
    buffer_1: [S; BUFFER_SIZE],
    buffer_2: [S; BUFFER_SIZE],
    descriptor_2: samd_dma::TransferDescriptor,
}

impl DmaResources<u8> {
    pub const fn new() -> Self {
        Self {
            buffer_1: [0; BUFFER_SIZE],
            buffer_2: [0; BUFFER_SIZE],
            descriptor_2: samd_dma::TransferDescriptor::new(),
        }
    }
}

pub type SampleBuffer<S> = &'static mut [S];

pub struct FillableBuffer<S: 'static + PrimInt> {
    pub period: Nanoseconds,
    pub amplitude: S,
    pub invert: bool,
    pub phase_offset: Nanoseconds,
    pub sample_period: Nanoseconds,
    buffer: SampleBuffer<S>,
}

impl<S: PrimInt> FillableBuffer<S> {
    fn calculate(&mut self) {
        for (i, sample) in self.buffer.iter_mut().enumerate() {
            let t = (self.phase_offset.0 + i as u32 * self.sample_period.0) % self.period.0;
            *sample = if (t > self.period.0 / 2) != self.invert {
                self.amplitude
            } else {
                S::zero()
            }
        }
    }

    pub fn fill(mut self) -> SampleBuffer<S> {
        self.calculate();
        self.buffer
    }
}

pub struct DacDriver<D: Dac> {
    dac: D,
    dma_channel: samd_dma::Channel,
    descriptor_2: &'static mut samd_dma::TransferDescriptor,
    period: Nanoseconds,
    amplitude: D::Amplitude,
    invert: bool,
    phase_offset: Nanoseconds,
    current_buffer: SampleBuffer<D::Amplitude>,
    filled_buffer: Option<SampleBuffer<D::Amplitude>>,
    first_descriptor: bool,
}

impl<D: Dac> DacDriver<D> {
    pub fn new(
        dac: D,
        mut dma_channel: samd_dma::Channel,
        dma_resources: &'static mut DmaResources<D::Amplitude>,
    ) -> Self {
        // Configure DMA channel
        // Only transfer one sample each time we are triggered
        dma_channel.set_trigger_action(samd_dma::TriggerAction::Beat);
        dma_channel.set_source(D::DMA_TRIGGER_SOURCE);
        dma_channel.enable_interrupts(samd_dma::Interrupts::TCMPL);

        let buffer_1 = &mut dma_resources.buffer_1;
        let buffer_2 = &mut dma_resources.buffer_2;

        let descriptor_1 = dma_channel.get_first_descriptor();
        let descriptor_2 = &mut dma_resources.descriptor_2;

        // Configure descriptors
        descriptor_1.set_beat_size(samd_dma::BeatSize::Byte);
        descriptor_2.set_beat_size(samd_dma::BeatSize::Byte);

        descriptor_1.set_step_size(samd_dma::StepSize::X1);
        descriptor_2.set_step_size(samd_dma::StepSize::X1);

        descriptor_1.set_step_selection(true);
        descriptor_2.set_step_selection(true);

        descriptor_1.set_src_addr_increment(true);
        descriptor_2.set_src_addr_increment(true);

        descriptor_1.set_dst_addr(dac.dma_ptr().cast());
        descriptor_2.set_dst_addr(dac.dma_ptr().cast());

        descriptor_1.set_dest_addr_increment(false);
        descriptor_2.set_dest_addr_increment(false);

        descriptor_1.set_block_action(samd_dma::BlockAction::Int);
        descriptor_2.set_block_action(samd_dma::BlockAction::Int);

        // Link descriptors into a loop
        descriptor_1.link_descriptor(descriptor_2);
        descriptor_2.link_descriptor(descriptor_1);

        // Set up the first buffer with the first descriptor
        descriptor_1.set_block_count(buffer_1.len() as u16);
        descriptor_1.set_src_addr(unsafe { buffer_1.as_mut_ptr().add(buffer_1.len()) } as *mut ());
        descriptor_1.set_valid();

        // Set up the second buffer with the second descriptor
        descriptor_2.set_block_count(buffer_2.len() as u16);
        descriptor_2.set_src_addr(unsafe { buffer_2.as_mut_ptr().add(buffer_2.len()) } as *mut ());
        descriptor_2.set_valid();

        dma_channel.enable();

        Self {
            dac,
            dma_channel,
            descriptor_2,
            period: 400.hz().into(),
            amplitude: D::Amplitude::zero(),
            invert: false,
            phase_offset: 0.ns(),
            current_buffer: buffer_1,
            filled_buffer: Some(buffer_2),
            first_descriptor: true,
        }
    }

    pub fn submit(&mut self, new_buffer: SampleBuffer<D::Amplitude>) {
        let next_descriptor = if self.first_descriptor {
            &mut *self.descriptor_2
        } else {
            self.dma_channel.get_first_descriptor()
        };

        // request() has already marked this descriptor as invalid

        // src address needs to point to end of buffer
        next_descriptor
            .set_src_addr(unsafe { new_buffer.as_mut_ptr().add(new_buffer.len()) } as *mut ());
        next_descriptor.set_valid();

        self.phase_offset = ((self.phase_offset.0
            + self.dac.sample_period().0 * new_buffer.len() as u32)
            % self.period.0)
            .ns();
        self.filled_buffer = Some(new_buffer);

        // Resume in case we underflowed
        self.dma_channel.resume();
    }

    pub fn request(&mut self) -> Option<FillableBuffer<D::Amplitude>> {
        let flags = self.dma_channel.get_interrupt_flags();
        self.dma_channel.clear_interrupt_flags(flags);

        if !flags.contains(samd_dma::Interrupts::TCMPL) {
            // DMA is still ongoing, can't request buffer to fill
            return None;
        }

        // The TCMPL flag should mean that current_buffer is finished and no longer being "aliased"
        // by the DMAC. This means it is safe to hand out a unique reference to it.

        let prev_descriptor = if self.first_descriptor {
            self.dma_channel.get_first_descriptor()
        } else {
            &mut *self.descriptor_2
        };

        // We are done with the old buffer, so invalidate it, even if we don't have a new one yet.
        prev_descriptor.set_invalid();

        if let Some(filled_buffer) = self.filled_buffer.take() {
            // We have a new buffer, which should be starting automatically right now. This function
            // just updates our bookkeeping and gives the previous buffer back to the caller to
            // fill.

            self.first_descriptor = !self.first_descriptor;
            let old_buffer = core::mem::replace(&mut self.current_buffer, filled_buffer);

            Some(FillableBuffer {
                period: self.period,
                amplitude: self.amplitude,
                invert: self.invert,
                phase_offset: self.phase_offset,
                sample_period: self.dac.sample_period(),
                buffer: old_buffer,
            })
        } else {
            None
        }
    }
}

impl<D: Dac> Driver for DacDriver<D> {
    fn set(&mut self, period: Nanoseconds, amplitude: u8, invert: bool) {
        self.period = period;
        self.amplitude =
            cast(amplitude as u32 * cast::<_, u32>(D::MAX_AMPLITUDE).unwrap() / u8::MAX as u32)
                .unwrap();
        self.invert = invert;
    }
}
