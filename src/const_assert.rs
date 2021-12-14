macro_rules! const_assert {
    ($($list:ident : $ty:ty),* => $expr:expr) => {{
        struct Assert<$(const $list: usize,)*>;
        impl<$(const $list: $ty,)*> Assert<$($list,)*> {
            const OK: u8 = 0 - !($expr) as u8;
        }
        Assert::<$($list,)*>::OK
    }};
    ($expr:expr) => {
        #[allow(unused)]
        const OK: u8 = 0 - !($expr) as u8;
    };
}

pub(crate) use const_assert;
