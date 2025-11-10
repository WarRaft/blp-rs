//! Extension for image crate: BLP format support

use crate::blp_image::BlpImage;
use image::error::ImageFormatHint;
use image::{ColorType, ExtendedColorType, ImageDecoder, ImageEncoder, ImageResult};

/// Decoder for BLP images
pub struct BlpImageDecoder<'a> {
    buf: &'a [u8],
}

impl<'a> BlpImageDecoder<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf }
    }
}

impl<'a> ImageDecoder for BlpImageDecoder<'a> {
    fn dimensions(&self) -> (u32, u32) {
        match BlpImage::from_buf(self.buf) {
            Ok(img) => (img.width, img.height),
            Err(_) => (0, 0),
        }
    }

    fn color_type(&self) -> ColorType {
        ColorType::Rgba8
    }

    fn read_image(self, buf: &mut [u8]) -> ImageResult<()> {
        let mut img = BlpImage::from_buf(self.buf).map_err(|e| image::ImageError::Decoding(image::error::DecodingError::new(ImageFormatHint::Name("blp".to_string()), e.to_string())))?;
        img.decode(self.buf, &[true, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false])
            .map_err(|e| image::ImageError::Decoding(image::error::DecodingError::new(ImageFormatHint::Name("blp".to_string()), e.to_string())))?;
        let pixels = img.mipmaps[0]
            .image
            .as_ref()
            .unwrap()
            .as_raw();
        if buf.len() != pixels.len() {
            return Err(image::ImageError::Parameter(image::error::ParameterError::from_kind(image::error::ParameterErrorKind::DimensionMismatch)));
        }
        buf.copy_from_slice(pixels);
        Ok(())
    }

    fn read_image_boxed(self: Box<Self>, buf: &mut [u8]) -> ImageResult<()> {
        (*self).read_image(buf)
    }
}

/// Encoder for BLP images
pub struct BlpImageEncoder<W: std::io::Write> {
    writer: W,
}

impl<W: std::io::Write> BlpImageEncoder<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }
}

impl<W: std::io::Write> ImageEncoder for BlpImageEncoder<W> {
    fn write_image(mut self, buf: &[u8], width: u32, height: u32, _color: ExtendedColorType) -> ImageResult<()> {
        let img = BlpImage::from_rgba(buf, width, height).map_err(|e| image::ImageError::Encoding(image::error::EncodingError::new(ImageFormatHint::Name("blp".to_string()), e.to_string())))?;
        let ctx = img
            .encode_blp(90, &[true, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false])
            .map_err(|e| image::ImageError::Encoding(image::error::EncodingError::new(ImageFormatHint::Name("blp".to_string()), e.to_string())))?;
        self.writer
            .write_all(&ctx.bytes)
            .map_err(image::ImageError::IoError)
    }
}
