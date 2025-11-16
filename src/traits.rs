use crate::error::error::BlpError;
use crate::blp::Frame;
use image::DynamicImage;

/// Trait to detect & parse headers cheaply for supported formats.
/// Implementations should be light and must not store the full payload (heavy bytes)
/// — only metadata and pointers into `AnyImage.frames` if needed.
pub trait FormatDetector: Sized {
    /// Cheap detection that must not allocate but look for format signatures.
    fn detect(buf: &[u8]) -> bool;

    /// Parse header and return a format-specific metadata container along with frames.
    fn parse_header(buf: &[u8]) -> Result<(Self, Vec<Frame>), BlpError>;
}

/// Trait for decoding format-specific data to DynamicImage.
/// Each format implementation provides its own logic for converting to a single image.
pub trait ImageDecoder {
    /// Decode the buffer into a DynamicImage.
    /// For BLP: decodes the first mipmap.
    /// For PSD: composites all layers.
    /// For GIF: returns the first frame.
    /// For standard images: returns the decoded image.
    fn into_dynamic(buf: &[u8]) -> Result<DynamicImage, BlpError>;
}
