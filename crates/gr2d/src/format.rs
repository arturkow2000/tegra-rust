use gr2d_reg::SBFMT0::{self, DIFMT, SIFMT};
use tock_registers::fields::FieldValue;

pub trait Gr2dFormat {
    const SIFMT: FieldValue<u32, SBFMT0::Register>;
    const DIFMT: FieldValue<u32, SBFMT0::Register>;
}

impl Gr2dFormat for colorspace::Rgba8888 {
    const SIFMT: FieldValue<u32, SBFMT0::Register> = SIFMT::RGBA8888;
    const DIFMT: FieldValue<u32, SBFMT0::Register> = DIFMT::RGBA8888;
}

impl Gr2dFormat for colorspace::Bgra8888 {
    const SIFMT: FieldValue<u32, SBFMT0::Register> = SIFMT::BGRA8888;
    const DIFMT: FieldValue<u32, SBFMT0::Register> = DIFMT::BGRA8888;
}
