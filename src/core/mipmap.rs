use image::RgbaImage;

#[derive(Debug, Default)]
pub struct Mipmap {
    pub width: u32,
    pub height: u32,
    pub image: Option<RgbaImage>,
    //
    pub offset: usize,
    pub length: usize,
}
