use colorspace::BitsPerPixel;
use gr2d_reg::CTRLMAIN;
use tock_registers::fields::FieldValue;

pub fn bpp_to_dstcd(bpp: BitsPerPixel) -> FieldValue<u32, CTRLMAIN::Register> {
    match bpp {
        BitsPerPixel::BPP8 => CTRLMAIN::DSTCD::BPP8,
        BitsPerPixel::BPP16 => CTRLMAIN::DSTCD::BPP16,
        BitsPerPixel::BPP32 => CTRLMAIN::DSTCD::BPP32,
    }
}
