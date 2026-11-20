use crate::{BlpError, Frame};
use image::DynamicImage;

/// Cheap detection and header parsing of an input format.
///
/// Implementations keep only metadata: what [`parse_header`](Self::parse_header)
/// returns points into the buffer, it doesn't copy pixels.
pub trait FormatDetector: Sized {
    /// Does the buffer start with this format's signature? Never allocates.
    fn detect(buf: &[u8]) -> bool;

    /// The format's metadata and its frames (mips for BLP, animation
    /// frames for GIF, one frame otherwise).
    fn parse_header(buf: &[u8]) -> Result<(Self, Vec<Frame>), BlpError>;
}

/// Decoding an input format to a single image.
pub trait ImageDecoder {
    /// The image a viewer shows first: BLP's base mip, a GIF's first
    /// frame, a PSD's composite, the image itself otherwise.
    fn into_dynamic(buf: &[u8]) -> Result<DynamicImage, BlpError>;
}
