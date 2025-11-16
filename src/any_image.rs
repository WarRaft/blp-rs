use crate::blp::{self, Blp, Frame};
use crate::error::error::BlpError;
use crate::traits::FormatDetector;
use crate::gif::Gif;
use crate::psd::PsdImage;
use image::GenericImageView;
use image::{DynamicImage, RgbaImage};

/// Mipmap generation options for BLP encoding.
///
/// Allows precise control over which mipmaps to generate. Options are evaluated
/// in priority order: `specific_mips` > `min_size` > `mip_count`.
///
/// # Examples
///
/// ```no_run
/// use blp::any_image::EncodeMipOptions;
///
/// // Generate first 4 mipmaps
/// let opts = EncodeMipOptions {
///     mip_count: Some(4),
///     ..Default::default()
/// };
///
/// // Stop generating when smallest side reaches 16px
/// let opts = EncodeMipOptions {
///     min_size: Some(16),
///     ..Default::default()
/// };
///
/// // Generate specific mipmaps only (e.g., mips 0, 2, 4)
/// let opts = EncodeMipOptions {
///     specific_mips: Some(vec![true, false, true, false, true]),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Default)]
pub struct EncodeMipOptions {
    /// Maximum number of mipmaps to generate (None = all possible mipmaps).
    /// Ignored if `specific_mips` is set.
    pub mip_count: Option<usize>,
    
    /// Minimum size of the smallest dimension before stopping mipmap generation.
    /// For example, `Some(16)` stops when width or height reaches 16px.
    /// Ignored if `specific_mips` is set.
    pub min_size: Option<u32>,
    
    /// Direct control: boolean array where `true` = generate this mip level.
    /// Length determines max mips. Overrides `mip_count` and `min_size`.
    /// Example: `vec![true, true, false, true]` generates mips 0, 1, and 3.
    pub specific_mips: Option<Vec<bool>>,
}

/// Encoding options for AnyImage export.
///
/// # Examples
///
/// ```no_run
/// use blp::any_image::{AnyImage, EncodeOptions, EncodeMipOptions};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let data = std::fs::read("image.png")?;
/// let img = AnyImage::from_buffer(&data)?;
///
/// // Export as PNG
/// let png_bytes = img.encode(&EncodeOptions::Png { compression: None })?;
///
/// // Export as JPEG with quality
/// let jpg_bytes = img.encode(&EncodeOptions::Jpeg { quality: 90 })?;
///
/// // Encode to BLP from any image format (PNG, JPEG, PSD, etc.)
/// let blp_bytes = img.encode(&EncodeOptions::Blp {
///     quality: 90,
///     mip_options: None, // Generate all mipmaps
///     raw: None,
/// })?;
///
/// // Encode to BLP with custom mipmap options
/// let blp_bytes = img.encode(&EncodeOptions::Blp {
///     quality: 90,
///     mip_options: Some(EncodeMipOptions {
///         mip_count: Some(4), // Only first 4 mipmaps
///         min_size: None,
///         specific_mips: None,
///     }),
///     raw: None,
/// })?;
///
/// // Extract raw JPEG from BLP mip 0 (for BLP sources only)
/// let blp_data = std::fs::read("image.blp")?;
/// let blp_img = AnyImage::from_buffer(&blp_data)?;
/// let raw_jpeg = blp_img.encode(&EncodeOptions::Blp {
///     quality: 90,
///     mip_options: None,
///     raw: Some(0), // Extract raw JPEG from mip 0
/// })?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub enum EncodeOptions {
    /// Export as PNG with optional compression level (0-9, None = default)
    Png { compression: Option<u8> },
    /// Export as JPEG with quality (0-100)
    Jpeg { quality: u8 },
    /// Export as BLP
    Blp { 
        quality: u8,
        /// Mipmap generation options (None = generate all possible mipmaps)
        mip_options: Option<EncodeMipOptions>,
        /// Extract raw JPEG from specific BLP mip (only for JPEG-based BLP sources)
        /// Requires source to be BLP. `mip_index` selects which mip to extract.
        raw: Option<usize>,
    },
}

impl Default for EncodeOptions {
    fn default() -> Self {
        EncodeOptions::Png { compression: None }
    }
}

/// AnyImage is a convenience wrapper that accepts an in-memory buffer of
/// unknown image format and exposes a small, user-friendly API.
///
/// Supported inputs: BLP (preferred detection), standard formats supported
/// by the `image` crate (PNG, JPEG, GIF, ...), and PSD (via `psd` crate).
///
/// # Examples
///
/// ```no_run
/// use blp::any_image::{AnyImage, EncodeOptions};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Load any image format
/// let data = std::fs::read("image.blp")?;
/// let img = AnyImage::from_buffer(&data)?;
///
/// // Get image dimensions
/// let (width, height) = img.dimensions();
/// println!("Image size: {}x{}", width, height);
///
/// // Encode to different formats
/// let png_bytes = img.encode(&EncodeOptions::Png { compression: None })?;
/// std::fs::write("output.png", png_bytes)?;
///
/// let jpg_bytes = img.encode(&EncodeOptions::Jpeg { quality: 90 })?;
/// std::fs::write("output.jpg", jpg_bytes)?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct AnyImage {
    /// Type-specific data (BLP headers, GIF frames metadata, PSD dims, etc.)
    pub data: AnyImageData,
    /// Original buffer given to the loader
    pub buf: Vec<u8>,
    /// Every image has frames (metainfo). For single-frame images this contains one entry.
    pub frames: Vec<Frame>,
}

/// Type-specific inner data for `AnyImage`.
#[derive(Debug, Clone)]
pub enum AnyImageData {
    Blp(Blp),      // header-only BLP representation
    Gif(Gif),      // GIF metadata; frames still in AnyImage.frames
    Psd(PsdImage), // PSD metadata; frames still in AnyImage.frames
    Image,         // regular single-frame image — frames[0] contains dims
}

/// Trait to detect & parse headers cheaply for supported formats.
/// Implementations should be light and must not store the payload (heavy bytes)
/// — only metadata and pointers into `AnyImage.frames` if needed.
// FormatDetector trait lives in `src/traits.rs` to avoid circular deps.
// `FormatDetector` now lives in `src/traits.rs` and is used by
// the modules (`blp`, `gif`, `psd`) to implement cheap detection + header parsing.

impl AnyImage {
    /// Try to build AnyImage from a byte buffer.
    /// BLP signature is checked first to avoid double-parsing; then `image` is tried,
    /// then PSD as a last fallback.
    pub fn from_buffer(buf: &[u8]) -> Result<Self, BlpError> {
        // Use trait-based detectors — explicit ordering matters (BLP first)
        if Blp::detect(buf) {
            let (blp_hdr, frames) = blp::parse_header(buf)?;
            return Ok(AnyImage { data: AnyImageData::Blp(blp_hdr), buf: buf.to_vec(), frames });
        }

        if Gif::detect(buf) {
            let (gif_meta, frames) = Gif::parse_header(buf)?;
            return Ok(AnyImage { data: AnyImageData::Gif(gif_meta), buf: buf.to_vec(), frames });
        }

        if PsdImage::detect(buf) {
            let (psd_meta, frames) = PsdImage::parse_header(buf)?;
            return Ok(AnyImage { data: AnyImageData::Psd(psd_meta), buf: buf.to_vec(), frames });
        }

        // Other image formats (single frame)
        if let Ok(dynimg) = image::load_from_memory(buf) {
            let (w, h) = dynimg.dimensions();
            let frame = Frame { width: w, height: h, offset: 0, length: buf.len() };
            return Ok(AnyImage { data: AnyImageData::Image, buf: buf.to_vec(), frames: vec![frame] });
        }

        Err(BlpError::new("unsupported-format"))
    }

    /// Return the image dimensions (width, height) in pixels.
    /// Compatible with the `image` crate's `GenericImageView::dimensions()` API.
    pub fn dimensions(&self) -> (u32, u32) {
        match &self.data {
            AnyImageData::Blp(b) => (b.width, b.height),
            _ => {
                let frame = self.frames.get(0);
                (
                    frame.map(|f| f.width).unwrap_or(0),
                    frame.map(|f| f.height).unwrap_or(0),
                )
            }
        }
    }

    /// Encode the image to bytes according to the specified options.
    pub fn encode(&self, opts: &EncodeOptions) -> Result<Vec<u8>, BlpError> {
        match opts {
            EncodeOptions::Png { compression } => {
                let img = self.clone().into_dynamic()?;
                let mut bytes = Vec::new();
                if compression.is_some() {
                    // Use custom compression settings
                    let encoder = image::codecs::png::PngEncoder::new_with_quality(
                        &mut bytes,
                        image::codecs::png::CompressionType::Default,
                        image::codecs::png::FilterType::Sub,
                    );
                    use image::ImageEncoder;
                    encoder.write_image(
                        img.as_bytes(),
                        img.width(),
                        img.height(),
                        img.color().into(),
                    )?;
                } else {
                    // Use default encoder
                    let encoder = image::codecs::png::PngEncoder::new(&mut bytes);
                    use image::ImageEncoder;
                    encoder.write_image(
                        img.as_bytes(),
                        img.width(),
                        img.height(),
                        img.color().into(),
                    )?;
                }
                Ok(bytes)
            }
            EncodeOptions::Jpeg { quality } => {
                let img = self.clone().into_dynamic()?;
                let rgb = img.to_rgb8();
                let mut bytes = Vec::new();
                let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, *quality);
                use image::ImageEncoder;
                encoder.write_image(
                    rgb.as_raw(),
                    rgb.width(),
                    rgb.height(),
                    image::ColorType::Rgb8.into(),
                )?;
                Ok(bytes)
            }
            EncodeOptions::Blp { quality, mip_options, raw } => {
                // Check if we need raw JPEG extraction from BLP
                if let Some(mip_index) = raw {
                    // Raw extraction only works for BLP sources
                    match &self.data {
                        AnyImageData::Blp(blp) => {
                            if blp.texture_type != blp::TextureType::JPEG {
                                return Err(BlpError::new("blp.raw-export-not-jpeg"));
                            }
                            let hdr = blp::shared_jpeg_header(&self.buf)
                                .ok_or_else(|| BlpError::new("jpeg.shared_header_missing"))?;
                            let mip = blp::mip_raw(&self.buf, *mip_index)
                                .ok_or_else(|| BlpError::new("jpeg.mip_missing"))?;
                            let mut out = Vec::with_capacity(hdr.len() + mip.len());
                            out.extend_from_slice(hdr);
                            out.extend_from_slice(mip);
                            return Ok(out);
                        }
                        _ => return Err(BlpError::new("blp.raw-export-requires-blp-source")),
                    }
                }

                // Generate BLP from any image format
                use crate::traits::ImageDecoder;
                let base_img = match &self.data {
                    AnyImageData::Blp(_) => Blp::to_dynamic(&self.buf)?,
                    AnyImageData::Gif(_) => Gif::to_dynamic(&self.buf)?,
                    AnyImageData::Psd(_) => PsdImage::to_dynamic(&self.buf)?,
                    AnyImageData::Image => image::load_from_memory(&self.buf)
                        .map_err(|_| BlpError::new("error-image-load"))?,
                };

                let rgba = base_img.to_rgba8();
                let (width, height) = rgba.dimensions();
                let raw_pixels = rgba.into_raw();

                // Generate mipmaps based on options
                let mip_visible = if let Some(opts) = mip_options {
                    // Use EncodeMipOptions to determine which mips to generate
                    if let Some(ref specific) = opts.specific_mips {
                        // Direct specification
                        specific.clone()
                    } else {
                        // Calculate based on mip_count or min_size
                        let max_mips = (32 - width.max(height).leading_zeros()) as usize;
                        let mut visible = vec![false; 16];
                        
                        let mut w = width;
                        let mut h = height;
                        for i in 0..max_mips.min(16) {
                            let should_generate = if let Some(min_sz) = opts.min_size {
                                w.min(h) >= min_sz
                            } else if let Some(count) = opts.mip_count {
                                i < count
                            } else {
                                true // Generate all
                            };
                            
                            visible[i] = should_generate;
                            if !should_generate {
                                break;
                            }
                            w = (w / 2).max(1);
                            h = (h / 2).max(1);
                        }
                        visible
                    }
                } else {
                    // Default: generate all possible mipmaps
                    vec![true; 16]
                };

                // Encode to BLP bytes
                blp::encode_rgba_to_blp(&raw_pixels, width, height, *quality, &mip_visible)
            }
        }
    }

    /// Convert into a single `DynamicImage`. For BLP this returns the first mip
    /// (decoded on demand). Consumes self.
    /// Decode and return the first frame as DynamicImage.
    pub fn into_dynamic(self) -> Result<DynamicImage, BlpError> {
        use crate::traits::ImageDecoder;
        match self.data {
            AnyImageData::Blp(_) => Blp::to_dynamic(&self.buf),
            AnyImageData::Gif(_) => Gif::to_dynamic(&self.buf),
            AnyImageData::Psd(_) => PsdImage::to_dynamic(&self.buf),
            AnyImageData::Image => image::load_from_memory(&self.buf)
                .map_err(|_| BlpError::new("error-image-load")),
        }
    }

    /// Produce a Vec of mipmaps (owned `RgbaImage`s).
    /// For BLP: return the full decoded mip chain. For regular images: generate
    /// a mip chain by successive downscaling of the base image.
    /// Decode and return all frames as RgbaImage (for multi-frame or mipmaps).
    pub fn decode_frames(&self) -> Result<Vec<RgbaImage>, BlpError> {
        match &self.data {
            AnyImageData::Blp(_) => blp::open_mipmaps(&self.buf),
            AnyImageData::Gif(_) => Gif::decode_frames(&self.buf),
            AnyImageData::Psd(_) | AnyImageData::Image => {
                let img = image::load_from_memory(&self.buf)?.to_rgba8();
                Ok(vec![img])
            }
        }
    }

    /// Decode a single frame by index. For BLP, it returns the selected mipmap.
    pub fn decode_frame(&self, idx: usize) -> Result<RgbaImage, BlpError> {
        match &self.data {
            AnyImageData::Blp(b) => {
                // Use BLP internal decoders targeted to the single index
                let frame = self
                    .frames
                    .get(idx)
                    .ok_or_else(|| BlpError::new("error-frame-oob").with_arg("idx", idx as u32))?;
                if frame.length == 0 {
                    return Err(BlpError::new("error-blp-no-mipmap"));
                }
                match b.texture_type {
                    blp::TextureType::JPEG => blp::decode::decode_jpeg_frame(b, frame, &self.buf),
                    blp::TextureType::PALETTE => blp::decode::decode_palette_frame(b, frame, &self.buf),
                }
            }
            AnyImageData::Gif(_) => {
                // use the gif module for GIF frames
                Gif::decode_frame(&self.buf, idx)
            }
            AnyImageData::Psd(_) => {
                // PSD metadata is stored in AnyImageData::Psd, but decode uses our psd module.
                // PSD is single-frame: use psd module
                PsdImage::decode_frame(&self.buf, idx)
            }
            AnyImageData::Image => {
                // single-frame
                let img = image::load_from_memory(&self.buf)?.to_rgba8();
                if idx == 0 { Ok(img) } else { Err(BlpError::new("error-frame-oob").with_arg("idx", idx as u32)) }
            }
        }
    }

    /// If this is a BLP, return metadata; otherwise None.
    /// Return parsed BLP header as the new `blp::Blp` struct if buffer is BLP.
    pub fn blp_meta(&self) -> Option<Blp> {
        match &self.data {
            AnyImageData::Blp(b) => Some(b.clone()),
            _ => None,
        }
    }

    /// For BLP images return the shared JPEG header slice if present.
    pub fn shared_jpeg_header(&self) -> Option<&[u8]> {
        match &self.data {
            AnyImageData::Blp(_) => blp::shared_jpeg_header(&self.buf),
            _ => None,
        }
    }

    /// For BLP images return raw mip payload for index.
    pub fn mip_raw(&self, idx: usize) -> Option<&[u8]> {
        match &self.data {
            AnyImageData::Blp(_) => blp::mip_raw(&self.buf, idx),
            _ => None,
        }
    }
}
