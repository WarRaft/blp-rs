use crate::blp::{self, Blp, Frame};
use crate::error::error::BlpError;
use crate::gif::Gif;
use crate::jpg::Jpg;
use crate::psd::PsdImage;
use crate::traits::FormatDetector;
use image::DynamicImage;
use image::GenericImageView;

/// Configuration for BLP mipmap generation.
///
/// You can either:
/// - Specify `mip_count` to generate a fixed number of mipmaps
/// - Specify `min_size` to generate mipmaps until the smallest dimension reaches this size
/// - Specify `specific_mips` to explicitly control which mip levels to generate
/// - Leave all `None` to generate all possible mipmaps
///
/// Color quantization settings (for palette-based compression):
/// - `quantize_colors`: Number of colors in palette (1-256, None = no quantization, use JPEG)
/// - `quantize_dither`: Enable dithering for better quality at lower color counts
#[derive(Debug, Clone, Default)]
pub struct EncodeMipOptions {
    /// Number of mipmap levels to generate (including base level)
    pub mip_count: Option<usize>,
    /// Minimum dimension (width or height) for mipmap generation
    pub min_size: Option<u32>,
    /// Explicit control: `vec[i] = true` means generate mip i
    pub specific_mips: Option<Vec<bool>>,
    /// Number of colors for palette quantization (1-256). If set, uses PALETTE texture type.
    /// If None, uses JPEG texture type.
    pub quantize_colors: Option<u8>,
    /// Enable dithering when quantizing colors (reduces banding artifacts)
    pub quantize_dither: bool,
}

impl EncodeMipOptions {
    /// Calculate mipmap visibility mask based on options.
    ///
    /// Returns a `Vec<bool>` of length 16 where true means the mipmap should be generated.
    pub fn calculate_mip_visible(&self, width: u32, height: u32) -> Vec<bool> {
        if let Some(ref specific) = self.specific_mips {
            // Direct specification
            return specific.clone();
        }

        // Calculate based on mip_count or min_size
        let max_mips = (32 - width.max(height).leading_zeros()) as usize;
        let mut visible = vec![false; 16];

        let mut w = width;
        let mut h = height;
        for i in 0..max_mips.min(16) {
            let should_generate = if let Some(min_sz) = self.min_size {
                w.min(h) >= min_sz
            } else if let Some(count) = self.mip_count {
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
///         quantize_colors: None,
///         quantize_dither: false,
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
/// Supported inputs: BLP (preferred detection), GIF, JPEG, PNG, PSD, and other
/// formats supported by the `image` crate. Also supports creating from raw RGBA buffers.
///
/// # Examples
///
/// ```no_run
/// use blp::any_image::{AnyImage, EncodeOptions};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Load any image format from file
/// let data = std::fs::read("image.blp")?;
/// let img = AnyImage::from_buffer(&data)?;
///
/// // Create from raw RGBA buffer
/// let width = 256u32;
/// let height = 256u32;
/// let rgba_data = vec![255u8; (width * height * 4) as usize]; // White image
/// let img_from_rgba = AnyImage::from_rgba(rgba_data, width, height)?;
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
    Jpg(Jpg), // JPEG metadata for detailed analysis
    RgbaBuffer { width: u32, height: u32 }, // Raw RGBA pixel buffer
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

        if Jpg::detect(buf) {
            let (jpg_meta, frames) = Jpg::parse_header(buf)?;
            return Ok(AnyImage { data: AnyImageData::Jpg(jpg_meta), buf: buf.to_vec(), frames });
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

    /// Create AnyImage from raw RGBA pixel buffer.
    ///
    /// # Arguments
    /// * `rgba` - Raw RGBA pixel data (4 bytes per pixel: R, G, B, A)
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    ///
    /// # Returns
    /// `AnyImage` with `AnyImageData::RgbaBuffer` variant
    pub fn from_rgba(rgba: Vec<u8>, width: u32, height: u32) -> Result<Self, BlpError> {
        if width == 0 || height == 0 {
            return Err(BlpError::new("error-image-empty")
                .with_arg("width", width)
                .with_arg("height", height));
        }
        
        let expected = (width as usize) * (height as usize) * 4;
        if rgba.len() != expected {
            return Err(BlpError::new("error-rgba-buffer-size")
                .with_arg("expected", expected)
                .with_arg("actual", rgba.len()));
        }

        let frame = Frame { width, height, offset: 0, length: rgba.len() };
        Ok(AnyImage {
            data: AnyImageData::RgbaBuffer { width, height },
            buf: rgba,
            frames: vec![frame],
        })
    }

    /// Return the image dimensions (width, height) in pixels.
    /// Compatible with the `image` crate's `GenericImageView::dimensions()` API.
    pub fn dimensions(&self) -> (u32, u32) {
        match &self.data {
            AnyImageData::Blp(b) => (b.width, b.height),
            AnyImageData::Jpg(j) => (j.width, j.height),
            AnyImageData::RgbaBuffer { width, height } => (*width, *height),
            _ => {
                let frame = self.frames.get(0);
                (frame.map(|f| f.width).unwrap_or(0), frame.map(|f| f.height).unwrap_or(0))
            }
        }
    }

    /// Encode the image to bytes according to the specified options.
    ///
    /// This is the single point of export for all image formats.
    ///
    /// # BLP Encoding Behavior
    ///
    /// When encoding to BLP format, the function automatically:
    /// - Converts the source image to RGBA
    /// - Scales to power-of-two dimensions (upscaling if necessary)
    /// - Generates mipmaps according to `mip_options`
    /// - Encodes all mipmaps with the specified quality
    ///
    /// For raw JPEG extraction from BLP sources, use `raw: Some(mip_index)`.
    pub fn encode(&self, opts: &EncodeOptions) -> Result<Vec<u8>, BlpError> {
        match opts {
            EncodeOptions::Png { compression } => {
                let img = self.clone().into_dynamic()?;
                let mut bytes = Vec::new();
                if compression.is_some() {
                    // Use custom compression settings
                    let encoder = image::codecs::png::PngEncoder::new_with_quality(&mut bytes, image::codecs::png::CompressionType::Default, image::codecs::png::FilterType::Sub);
                    use image::ImageEncoder;
                    encoder.write_image(img.as_bytes(), img.width(), img.height(), img.color().into())?;
                } else {
                    // Use default encoder
                    let encoder = image::codecs::png::PngEncoder::new(&mut bytes);
                    use image::ImageEncoder;
                    encoder.write_image(img.as_bytes(), img.width(), img.height(), img.color().into())?;
                }
                Ok(bytes)
            }
            EncodeOptions::Jpeg { quality } => {
                let img = self.clone().into_dynamic()?;
                let rgb = img.to_rgb8();
                let mut bytes = Vec::new();
                let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, *quality);
                use image::ImageEncoder;
                encoder.write_image(rgb.as_raw(), rgb.width(), rgb.height(), image::ColorType::Rgb8.into())?;
                Ok(bytes)
            }
            EncodeOptions::Blp { quality, mip_options, raw } => {
                // Check if we need raw JPEG extraction from BLP
                if let Some(mip_index) = raw {
                    // Raw extraction only works for BLP sources
                    return match &self.data {
                        AnyImageData::Blp(blp) => {
                            if blp.texture_type != blp::TextureType::JPEG {
                                return Err(BlpError::new("blp.raw-export-not-jpeg"));
                            }
                            let hdr = blp::header_data(&self.buf).ok_or_else(|| BlpError::new("jpeg.shared_header_missing"))?;
                            let mip = blp::mip_raw(&self.buf, *mip_index).ok_or_else(|| BlpError::new("jpeg.mip_missing"))?;
                            let mut out = Vec::with_capacity(hdr.len() + mip.len());
                            out.extend_from_slice(hdr);
                            out.extend_from_slice(mip);
                            Ok(out)
                        }
                        _ => Err(BlpError::new("blp.raw-export-requires-blp-source")),
                    };
                }

                // Convert to RGBA pixels
                let base_img = self.clone().into_dynamic()?;
                let rgba = base_img.to_rgba8();
                let (width, height) = rgba.dimensions();

                // Calculate mip visibility mask based on options
                let mip_visible = if let Some(opts) = mip_options {
                    opts.calculate_mip_visible(width, height)
                } else {
                    // Default: generate all possible mipmaps
                    vec![true; 16]
                };

                // Encode to BLP from RGBA pixels
                Blp::encode_from_rgba(rgba.as_raw(), width, height, *quality, &mip_visible)
            }
        }
    }

    /// Convert into a single `DynamicImage`. For BLP this returns the first mip
    /// (decoded on demand). Consumes self.
    /// Decode and return the first frame as DynamicImage.
    pub fn into_dynamic(self) -> Result<DynamicImage, BlpError> {
        use crate::traits::ImageDecoder;
        match self.data {
            AnyImageData::Blp(_) => Blp::into_dynamic(&self.buf),
            AnyImageData::Gif(_) => Gif::into_dynamic(&self.buf),
            AnyImageData::Jpg(_) => Jpg::into_dynamic(&self.buf),
            AnyImageData::Psd(_) => PsdImage::into_dynamic(&self.buf),
            AnyImageData::RgbaBuffer { width, height } => {
                image::RgbaImage::from_raw(width, height, self.buf)
                    .map(DynamicImage::ImageRgba8)
                    .ok_or_else(|| BlpError::new("error-rgba-image-creation"))
            }
            AnyImageData::Image => image::load_from_memory(&self.buf).map_err(|_| BlpError::new("error-image-load")),
        }
    }
}
