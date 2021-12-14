use crate::const_assert::const_assert;
use core::ops::Deref;

pub struct RegisterBlock<TCC, const CH_ID: u8>
where
    TCC: Deref<Target = crate::pac::tcc0::RegisterBlock>,
{
    tcc: TCC,
}

impl<TCC, const CH_ID: u8> RegisterBlock<TCC, CH_ID>
where
    TCC: Deref<Target = crate::pac::tcc0::RegisterBlock>,
{
    const_assert!(CH_ID < 4);

    const NUM_CHANNELS: u8 = 4;

    /// Safety: this should be safe as long as no two RegisterBlock structs exist with the same
    /// channel ID.
    pub unsafe fn new(tcc: &TCC) -> Self {
        Self {
            tcc: core::ptr::read(tcc),
        }
    }

    pub fn syncbusy_cc(&self) -> crate::pac::tcc0::syncbusy::CC0_R {
        let syncbusy = self.tcc.syncbusy.read();
        match CH_ID {
            0 => syncbusy.cc0(),
            1 => syncbusy.cc1(),
            2 => syncbusy.cc2(),
            3 => syncbusy.cc3(),
            _ => unreachable!(),
        }
    }

    pub fn intflag_mc(&self) -> crate::pac::tcc0::intflag::MC0_R {
        let intflag = self.tcc.intflag.read();
        match CH_ID {
            0 => intflag.mc0(),
            1 => intflag.mc1(),
            2 => intflag.mc2(),
            3 => intflag.mc3(),
            _ => unreachable!(),
        }
    }

    pub fn intflag_mc_clear(&self) {
        self.tcc.intflag.write(|w| match CH_ID {
            0 => w.mc0().set_bit(),
            1 => w.mc1().set_bit(),
            2 => w.mc2().set_bit(),
            3 => w.mc3().set_bit(),
            _ => unreachable!(),
        });
    }

    pub fn cc(&self) -> &crate::pac::tcc0::CC {
        &self.tcc.cc()[CH_ID as usize]
    }

    pub fn ccb(&self) -> &crate::pac::tcc0::CCB {
        &self.tcc.ccb()[CH_ID as usize]
    }
}
