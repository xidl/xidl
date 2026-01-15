pub(crate) trait ToNeBytes<const N: usize>: Sized {
    fn to_ne_bytes(&self) -> [u8; N];
}

pub(crate) trait FromBytes<const N: usize>: Sized {
    fn from_le_bytes(bytes: [u8; N]) -> Self;
    fn from_be_bytes(bytes: [u8; N]) -> Self;
}

macro_rules! impl_to_ne_bytes {
    ($($ty:ty, $len:literal)*) => {
        $(
            impl ToNeBytes<$len> for $ty {
                #[inline(always)]
                fn to_ne_bytes(&self) -> [u8; $len] {
                    <$ty>::to_ne_bytes(*self)
                }
            }
        )*
    };
}

macro_rules! impl_from_bytes {
    ($($ty:ty, $len:literal)*) => {
        $(
            impl FromBytes<$len> for $ty {
                #[inline(always)]
                fn from_le_bytes(bytes: [u8; $len]) -> Self {
                    <$ty>::from_le_bytes(bytes)
                }

                #[inline(always)]
                fn from_be_bytes(bytes: [u8; $len]) -> Self {
                    <$ty>::from_be_bytes(bytes)
                }
            }
        )*
    };
}

impl_to_ne_bytes!(
    u8, 1
    u16, 2
    u32, 4
    u64, 8
    u128, 16
    i8, 1
    i16, 2
    i32, 4
    i64, 8
    i128, 16
    f32, 4
    f64, 8
);

impl_from_bytes!(
    u8, 1
    i8, 1
    u16, 2
    u32, 4
    u64, 8
    i16, 2
    i32, 4
    i64, 8
    f32, 4
    f64, 8
);

impl ToNeBytes<1> for bool {
    fn to_ne_bytes(&self) -> [u8; 1] {
        <u8>::to_ne_bytes(*self as u8)
    }
}

impl FromBytes<1> for bool {
    fn from_le_bytes(bytes: [u8; 1]) -> Self {
        <u8>::from_le_bytes(bytes) != 0
    }
    fn from_be_bytes(bytes: [u8; 1]) -> Self {
        <u8>::from_be_bytes(bytes) != 0
    }
}
