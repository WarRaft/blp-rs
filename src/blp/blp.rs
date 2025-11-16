use crate::BlpError;
use num_enum::TryFromPrimitive;

/// Maximum number of mipmaps supported by the BLP format.
pub(crate) const MAX_MIPS: usize = 16;

/// Size of the BLP header in bytes.
pub(crate) const HEADER_SIZE: u64 = 156;

/// BLP file structure containing header information.
#[derive(Debug, Clone)]
pub struct Blp {
    /// BLP version (BLP0, BLP1, or BLP2)
    pub version: Version,
    /// Texture compression type
    pub texture_type: TextureType,
    /// Compression value
    pub compression: u8,
    /// Number of alpha bits
    pub alpha_bits: u32,
    /// Alpha type
    pub alpha_type: u8,
    /// Has mipmaps flag
    pub has_mips: u8,
    /// Base texture width in pixels
    pub width: u32,
    /// Base texture height in pixels
    pub height: u32,
    /// Extra field (meaningful only for BLP1)
    pub extra: u32,
    /// Has mipmaps field (meaningful for BLP1/BLP2)
    pub has_mipmaps: u32,
    /// Number of holes
    pub holes: usize,
    /// Header data frame
    pub header: Frame,
}

/// BLP version identifier.
#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd, Eq, TryFromPrimitive)]
#[repr(u32)]
pub enum Version {
    BLP0 = 0x424C5030, // "BLP0"
    #[default]
    BLP1 = 0x424C5031, // "BLP1"
    BLP2 = 0x424C5032, // "BLP2"
}

/// Texture compression type.
#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd, Eq, TryFromPrimitive)]
#[repr(u32)]
pub enum TextureType {
    #[default]
    JPEG = 0,
    PALETTE = 1,
}

/// Represents a single mipmap frame.
#[derive(Debug, Default, Clone)]
pub struct Frame {
    /// Frame width in pixels
    pub width: u32,
    /// Frame height in pixels
    pub height: u32,
    /// Byte offset in the file
    pub offset: usize,
    /// Data length in bytes
    pub length: usize,
}

impl crate::traits::FormatDetector for Blp {
    fn detect(buf: &[u8]) -> bool {
        buf.len() >= 3 && &buf[0..3] == b"BLP"
    }

    fn parse_header(buf: &[u8]) -> Result<(Self, Vec<Frame>), BlpError> {
        crate::blp::parse::parse_header(buf)
    }
}
