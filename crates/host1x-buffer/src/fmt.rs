const fn fmt(id: u32, bpp: u32, compressed: u32, align_bytes: u32, tex_w: u32, tex_h: u32) -> u32 {
    tex_h << 26 | tex_w << 23 | align_bytes << 16 | !!compressed << 14 | id << 8 | bpp
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
#[repr(u32)]
pub enum Format {
    RGB565 = fmt(4, 16, 0, 64, 1, 1),
    RGBA8888 = fmt(9, 32, 0, 64, 1, 1),
    BGRA8888 = fmt(10, 32, 0, 64, 1, 1),
}
