use crate::{Encoding, Rgb, RgbAlpha};

pub struct Color<E: Encoding + ?Sized>(pub(crate) E::Storage);
impl<E: Rgb> Color<E> {
    // TODO: make this const
    // we need https://github.com/rust-lang/rust/issues/143874
    // and we need arbitrary_int to support that
    pub fn rgb(r: E::R, g: E::G, b: E::B) -> Self {
        let mut c = E::BLACK.0;
        E::set_r(&mut c, r);
        E::set_g(&mut c, g);
        E::set_b(&mut c, b);
        Self(c)
    }

    #[inline]
    pub fn r(&self) -> E::R {
        E::r(&self.0)
    }

    #[inline]
    pub fn g(&self) -> E::G {
        E::g(&self.0)
    }

    #[inline]
    pub fn b(&self) -> E::B {
        E::b(&self.0)
    }
}

impl<E: RgbAlpha> Color<E> {
    // TODO: make this const
    // we need https://github.com/rust-lang/rust/issues/143874
    // and we need arbitrary_int to support that
    pub fn rgba(r: E::R, g: E::G, b: E::B, a: E::A) -> Self {
        let mut c = E::BLACK.0;
        E::set_r(&mut c, r);
        E::set_g(&mut c, g);
        E::set_b(&mut c, b);
        E::set_a(&mut c, a);
        Self(c)
    }

    #[inline]
    pub fn a(&self) -> E::A {
        E::a(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use arbitrary_int::{
        prelude::{u5, u6},
        traits::Integer,
    };

    use crate::{NoAlpha, RgbGeneric};

    use super::Color;

    #[test]
    fn test_color() {
        let c = Color::<RgbGeneric<u5, u6, u5, NoAlpha>>::rgb(u5::new(12), u6::new(0), u5::MAX);
        assert_eq!((c.r(), c.g(), c.b()), (u5::new(12), u6::default(), u5::MAX));

        let c = Color::<RgbGeneric<u8, u8, u8, u8>>::rgba(120, 42, 78, 179);
        assert_eq!((c.r(), c.g(), c.b(), c.a()), (120, 42, 78, 179));
    }
}
