//! [`AnyImage`]: any supported input behind one API.

#[cfg(feature = "psd")]
pub use crate::psd::PsdImage;
use crate::{Blp, BlpError, FormatDetector, Frame, ImageDecoder};
pub use crate::{gif::Gif, jpg::Jpg};
use image::{DynamicImage, ImageEncoder, ImageReader, RgbaImage};
use std::io::Cursor;

/// Which mips to produce when encoding BLP. The first option set wins:
/// `specific_mips`, then `min_size`, then `mip_count`; none: all of them.
#[derive(Debug, Clone, Default)]
pub struct EncodeMipOptions {
    /// Mips in total, the base one included.
    pub mip_count: Option<usize>,
    /// Stop before the smaller side drops under this many pixels.
    pub min_size: Option<u32>,
    /// `specific_mips[i]`: produce mip `i`; missing entries are produced.
    pub specific_mips: Option<Vec<bool>>,
}

impl EncodeMipOptions {
    /// One flag per mip slot for a base mip of `width` × `height` (the
    /// size after scaling up to powers of two).
    pub fn calculate_mip_visible(&self, width: u32, height: u32) -> Vec<bool> {
        if let Some(specific) = &self.specific_mips {
            return specific.clone();
        }
        let levels = (u32::BITS - width.max(height).leading_zeros()) as usize;
        (0..crate::blp::MAX_MIPS)
            .map(|i| {
                i < levels
                    && match (self.min_size, self.mip_count) {
                        (Some(min), _) => {
                            (width >> i)
                                .max(1)
                                .min((height >> i).max(1))
                                >= min
                        }
                        (None, Some(count)) => i < count,
                        (None, None) => true,
                    }
            })
            .collect()
    }
}

/// What [`AnyImage::encode`] produces.
///
/// ```no_run
/// use blp::{AnyImage, EncodeMipOptions, EncodeOptions};
///
/// let img = AnyImage::from_buffer(&std::fs::read("image.png")?)?;
/// let png = img.encode(&EncodeOptions::Png { compression: None })?;
/// let jpg = img.encode(&EncodeOptions::Jpeg { quality: 90 })?;
/// let blp = img.encode(&EncodeOptions::Blp {
///     quality: 90,
///     mip_options: Some(EncodeMipOptions { mip_count: Some(4), ..Default::default() }),
///     raw: None,
/// })?;
/// # Ok::<(), blp::BlpError>(())
/// ```
#[derive(Debug, Clone)]
pub enum EncodeOptions {
    /// PNG; `compression` 0–9 (fast to best), `None` for the default.
    Png { compression: Option<u8> },
    /// JPEG at `quality` 0–100.
    Jpeg { quality: u8 },
    /// BLP1 JPEG (feature `encode`), or with `raw: Some(i)` the JPEG of a
    /// JPEG BLP's mip `i` as stored, without re-encoding.
    Blp { quality: u8, mip_options: Option<EncodeMipOptions>, raw: Option<usize> },
}

impl Default for EncodeOptions {
    fn default() -> Self {
        EncodeOptions::Png { compression: None }
    }
}

/// An image of any supported format: its metadata, its frames (BLP mips,
/// GIF frames, or the one image), and the buffer it was read from.
///
/// ```no_run
/// use blp::{AnyImage, EncodeOptions};
///
/// let img = AnyImage::from_buffer(&std::fs::read("texture.blp")?)?;
/// let (w, h) = img.dimensions();
/// for (i, mip) in img.decode_frames()?.iter().enumerate() {
///     mip.save(format!("mip_{i}.png"))?;
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone)]
pub struct AnyImage {
    pub data: AnyImageData,
    pub buf: Vec<u8>,
    pub frames: Vec<Frame>,
}

/// The format of an [`AnyImage`] and its metadata.
#[derive(Debug, Clone)]
pub enum AnyImageData {
    Blp(Blp),
    Gif(Gif),
    #[cfg(feature = "psd")]
    Psd(PsdImage),
    Jpg(Jpg),
    /// Raw RGBA pixels (from [`AnyImage::from_rgba`]).
    RgbaBuffer {
        width: u32,
        height: u32,
    },
    /// Any other format the `image` crate reads.
    Image,
}

impl AnyImage {
    /// Reads the format and metadata; pixels are decoded on demand.
    pub fn from_buffer(buf: &[u8]) -> Result<Self, BlpError> {
        let (data, frames) = if Blp::detect(buf) {
            let (blp, frames) = Blp::parse(buf)?;
            (AnyImageData::Blp(blp), frames)
        } else if Gif::detect(buf) {
            let (gif, frames) = Gif::parse_header(buf)?;
            (AnyImageData::Gif(gif), frames)
        } else if Jpg::detect(buf) {
            let (jpg, frames) = Jpg::parse_header(buf)?;
            (AnyImageData::Jpg(jpg), frames)
        } else if let Some(psd) = Self::psd(buf) {
            psd?
        } else {
            let reader = ImageReader::new(Cursor::new(buf)).with_guessed_format()?;
            if reader.format().is_none() {
                return Err(BlpError::new("unsupported-format"));
            }
            let (width, height) = reader.into_dimensions()?;
            (AnyImageData::Image, vec![Frame { width, height, offset: 0, length: buf.len() }])
        };
        Ok(AnyImage { data, buf: buf.to_vec(), frames })
    }

    #[cfg(feature = "psd")]
    fn psd(buf: &[u8]) -> Option<Result<(AnyImageData, Vec<Frame>), BlpError>> {
        PsdImage::detect(buf).then(|| PsdImage::parse_header(buf).map(|(psd, frames)| (AnyImageData::Psd(psd), frames)))
    }

    #[cfg(not(feature = "psd"))]
    fn psd(_: &[u8]) -> Option<Result<(AnyImageData, Vec<Frame>), BlpError>> {
        None
    }

    /// Wraps raw RGBA pixels (4 bytes per pixel, rows top to bottom).
    pub fn from_rgba(rgba: Vec<u8>, width: u32, height: u32) -> Result<Self, BlpError> {
        if width == 0 || height == 0 {
            return Err(BlpError::new("error-image-empty")
                .with_arg("width", width)
                .with_arg("height", height));
        }
        let expected = width as usize * height as usize * 4;
        if rgba.len() != expected {
            return Err(BlpError::new("error-rgba-buffer-size")
                .with_arg("expected", expected)
                .with_arg("actual", rgba.len()));
        }
        let frames = vec![Frame { width, height, offset: 0, length: rgba.len() }];
        Ok(AnyImage { data: AnyImageData::RgbaBuffer { width, height }, buf: rgba, frames })
    }

    /// The size of the image (BLP: of its base mip), in pixels.
    pub fn dimensions(&self) -> (u32, u32) {
        match &self.data {
            AnyImageData::Blp(b) => (b.width, b.height),
            AnyImageData::Jpg(j) => (j.width, j.height),
            AnyImageData::RgbaBuffer { width, height } => (*width, *height),
            _ => self
                .frames
                .first()
                .map_or((0, 0), |f| (f.width, f.height)),
        }
    }

    /// Decodes frame `index`: a BLP's mip, a GIF's frame; other formats
    /// only have frame 0.
    pub fn decode_frame(&self, index: usize) -> Result<RgbaImage, BlpError> {
        let frame = self.frames.get(index).ok_or_else(|| {
            BlpError::new("frame.oob")
                .with_arg("index", index)
                .with_arg("frames", self.frames.len())
        })?;
        match &self.data {
            AnyImageData::Blp(blp) => blp.decode(frame, &self.buf),
            AnyImageData::Gif(_) => Ok(Gif::decode_frames(&self.buf)?.swap_remove(index)),
            _ => Ok(self.to_dynamic()?.into_rgba8()),
        }
    }

    /// Decodes every frame, in order.
    pub fn decode_frames(&self) -> Result<Vec<RgbaImage>, BlpError> {
        match &self.data {
            AnyImageData::Gif(_) => Gif::decode_frames(&self.buf),
            _ => (0..self.frames.len())
                .map(|i| self.decode_frame(i))
                .collect(),
        }
    }

    /// The image a viewer shows first: a BLP's base mip, a GIF's first
    /// frame, a PSD's composite, the image itself otherwise.
    pub fn to_dynamic(&self) -> Result<DynamicImage, BlpError> {
        match &self.data {
            AnyImageData::Blp(_) => Blp::into_dynamic(&self.buf),
            AnyImageData::Gif(_) => Gif::into_dynamic(&self.buf),
            AnyImageData::Jpg(_) => Jpg::into_dynamic(&self.buf),
            #[cfg(feature = "psd")]
            AnyImageData::Psd(_) => PsdImage::into_dynamic(&self.buf),
            AnyImageData::RgbaBuffer { width, height } => RgbaImage::from_raw(*width, *height, self.buf.clone())
                .map(DynamicImage::ImageRgba8)
                .ok_or_else(|| BlpError::new("error-rgba-image-creation")),
            AnyImageData::Image => Ok(image::load_from_memory(&self.buf)?),
        }
    }

    /// [`to_dynamic`](Self::to_dynamic), consuming the image.
    pub fn into_dynamic(self) -> Result<DynamicImage, BlpError> {
        self.to_dynamic()
    }

    /// Encodes the image (see [`to_dynamic`](Self::to_dynamic)) in another
    /// format. BLP output is scaled up to power-of-two sides and gets the
    /// mips `mip_options` asks for.
    pub fn encode(&self, opts: &EncodeOptions) -> Result<Vec<u8>, BlpError> {
        let mut out = Vec::new();
        match opts {
            EncodeOptions::Png { compression } => {
                use image::codecs::png::{CompressionType, FilterType, PngEncoder};
                let img = self.to_dynamic()?;
                let level = match compression {
                    None => CompressionType::Default,
                    Some(0..=3) => CompressionType::Fast,
                    Some(4..=6) => CompressionType::Default,
                    Some(_) => CompressionType::Best,
                };
                PngEncoder::new_with_quality(&mut out, level, FilterType::Adaptive).write_image(img.as_bytes(), img.width(), img.height(), img.color().into())?;
            }
            EncodeOptions::Jpeg { quality } => {
                let rgb = self.to_dynamic()?.into_rgb8();
                image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, *quality).write_image(rgb.as_raw(), rgb.width(), rgb.height(), image::ExtendedColorType::Rgb8)?;
            }
            EncodeOptions::Blp { raw: Some(mip), .. } => {
                let AnyImageData::Blp(blp) = &self.data else {
                    return Err(BlpError::new("blp.raw-export-requires-blp-source"));
                };
                if blp.encoding()? != crate::Encoding::Jpeg {
                    return Err(BlpError::new("blp.raw-export-not-jpeg"));
                }
                let frame = self
                    .frames
                    .get(*mip)
                    .filter(|f| f.length > 0)
                    .ok_or_else(|| BlpError::new("blp.mip.missing").with_arg("mip", *mip))?;
                out.extend_from_slice(blp.header_data(&self.buf)?);
                out.extend_from_slice(frame.data(&self.buf)?);
            }
            EncodeOptions::Blp { quality, mip_options, raw: None } => return self.encode_blp(*quality, mip_options.as_ref()),
        }
        Ok(out)
    }

    #[cfg(feature = "encode")]
    fn encode_blp(&self, quality: u8, mip_options: Option<&EncodeMipOptions>) -> Result<Vec<u8>, BlpError> {
        let rgba = self.to_dynamic()?.into_rgba8();
        let (w, h) = rgba.dimensions();
        let visible = mip_options.map_or_else(|| vec![true; crate::blp::MAX_MIPS], |o| o.calculate_mip_visible(w.next_power_of_two(), h.next_power_of_two()));
        Blp::encode_from_rgba(rgba.as_raw(), w, h, quality, &visible)
    }

    #[cfg(not(feature = "encode"))]
    fn encode_blp(&self, _: u8, _: Option<&EncodeMipOptions>) -> Result<Vec<u8>, BlpError> {
        Err(BlpError::new("blp.encode.disabled"))
    }
}
