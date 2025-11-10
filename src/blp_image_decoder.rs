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

impl<W: std::io::Write> BlpImageEncoder<W> {
    /// Write an explicit set of mipmaps into a BLP. The first image in `mips`
    /// is treated as the base level; subsequent entries correspond to lower
    /// mip levels. The number of mipmaps is capped at `MAX_MIPS`.
    pub fn write_mipmaps(mut self, mips: &[RgbaImage]) -> ImageResult<()> {
        if mips.is_empty() {
            return Err(image::ImageError::Parameter(image::error::ParameterError::from_kind(image::error::ParameterErrorKind::DimensionMismatch)));
        }

        // Use first mip as base to construct BlpImage
        let base = &mips[0];
        let buf = base.as_raw();
        let mut img = BlpImage::from_rgba(buf, base.width(), base.height()).map_err(|e| image::ImageError::Encoding(image::error::EncodingError::new(ImageFormatHint::Name("blp".to_string()), e.to_string())))?;

        // Fill subsequent mipmaps (up to MAX_MIPS) and validate sizes
        for (i, mip) in mips.iter().enumerate().take(MAX_MIPS) {
            if i == 0 {
                continue;
            } // already used
            if i >= img.mipmaps.len() {
                break;
            }
            let expected_w = img.mipmaps[i].width;
            let expected_h = img.mipmaps[i].height;
            if mip.width() != expected_w || mip.height() != expected_h {
                return Err(image::ImageError::Parameter(image::error::ParameterError::from_kind(image::error::ParameterErrorKind::DimensionMismatch)));
            }
            img.mipmaps[i].image = Some(mip.clone());
        }

        let ctx = img
            .encode_blp(90, &[true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true])
            .map_err(|e| image::ImageError::Encoding(image::error::EncodingError::new(ImageFormatHint::Name("blp".to_string()), e.to_string())))?;
        self.writer
            .write_all(&ctx.bytes)
            .map_err(image::ImageError::IoError)
    }
}
