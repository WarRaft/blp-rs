use crate::error::error::BlpError;

/// Trait to detect & parse headers cheaply for supported formats.
/// Implementations should be light and must not store the full payload (heavy bytes)
/// — only metadata and pointers into `AnyImage.frames` if needed.
pub trait FormatDetector: Sized {
    /// Cheap detection that must not allocate but look for format signatures.
    fn detect(buf: &[u8]) -> bool;

    /// Parse header and return a format-specific metadata container.
    fn parse_header(buf: &[u8]) -> Result<Self, BlpError>;
}
