use crate::{BlpImage, MAX_MIPS};
use image::error::ImageFormatHint;
use image::{ExtendedColorType, ImageEncoder, ImageResult, RgbaImage};

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
