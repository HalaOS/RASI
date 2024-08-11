#[allow(unused)]
#[derive(Clone, Copy, Debug)]
pub struct Radix<const MAX: u32>(pub u32);

use quickcheck::{Arbitrary, Gen};

impl<const MAX: u32> Arbitrary for Radix<MAX> {
    fn arbitrary(g: &mut Gen) -> Self {
        let radix = (u32::arbitrary(g) % (MAX - 2)) + 2;
        Self(radix)
    }
}
