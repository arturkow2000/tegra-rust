use arbitrary_int::traits::Integer;
use colorspace::{Format, Rgb, color::Color};
use embedded_graphics::{
    geometry::{Point, Size},
    primitives::Rectangle,
};
use gr2d::format::Gr2dFormat;
use host1x_core::queue::Queue as _;
use host1x_drm::{
    buffer::PixelBuffer,
    fence::Syncobj,
    mapping::{MappingMode, ReadWrite},
    uapi::v2::{channel::mapping::BufferMapping, queue::Queue},
};

pub struct App {}
impl App {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn render<'buf, F: Format + Gr2dFormat, CMM: MappingMode>(
        &self,
        queue: &Queue<gr2d_reg::Client>,
        fence: &mut Syncobj,
        buf: BufferMapping<'buf, gr2d_reg::Client, PixelBuffer<F>, CMM, ReadWrite>,
    ) -> BufferMapping<'buf, gr2d_reg::Client, PixelBuffer<F>, CMM, ReadWrite>
    where
        F::Encoding: Rgb,
    {
        queue
            .submit(
                fence,
                gr2d::fill_rect(
                    buf,
                    Color::rgb(
                        <<F::Encoding as Rgb>::R as Integer>::MIN,
                        <<F::Encoding as Rgb>::G as Integer>::MAX,
                        <<F::Encoding as Rgb>::B as Integer>::MIN,
                    ),
                    Rectangle {
                        top_left: Point::new(10, 10),
                        size: Size::new(100, 100),
                    },
                ),
            )
            .await
            .unwrap()
            .await
            .unwrap()
    }
}
