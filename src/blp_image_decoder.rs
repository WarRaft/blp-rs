use crate::_blp_image::BlpImage;
use crate::_blp_image::MAX_MIPS;
use image::RgbaImage;
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

impl<'a> BlpImageDecoder<'a> {
    /// Decode and return all available mipmaps as owned `RgbaImage`s.
    /// Mipmaps without image data are skipped.
    pub fn into_mipmaps(self) -> ImageResult<Vec<RgbaImage>> {
        let mut img = BlpImage::from_buf(self.buf).map_err(|e| image::ImageError::Decoding(image::error::DecodingError::new(ImageFormatHint::Name("blp".to_string()), e.to_string())))?;
        img.decode(self.buf, &[true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true])
            .map_err(|e| image::ImageError::Decoding(image::error::DecodingError::new(ImageFormatHint::Name("blp".to_string()), e.to_string())))?;

        let mut out = Vec::new();
        for m in img.mipmaps.into_iter().take(MAX_MIPS) {
            if let Some(rgba) = m.image {
                out.push(rgba);
            }
        }
        Ok(out)
    }
}