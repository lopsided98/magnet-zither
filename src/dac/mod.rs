// SPDX-License-Identifier: GPL-3.0-or-later
use crate::hal::time::Nanoseconds;
use num_traits::PrimInt;

pub trait Dac: DacDmaTrigger {
    type Amplitude: 'static + PrimInt;
    const MAX_AMPLITUDE: Self::Amplitude;

    fn set_amplitude(&mut self, amplitude: Self::Amplitude);

    fn sample_period(&self) -> Nanoseconds;

    fn dma_ptr(&self) -> *mut Self::Amplitude;
}

pub trait DacDmaTrigger {
    const DMA_TRIGGER_SOURCE: samd_dma::TriggerSource;
}
