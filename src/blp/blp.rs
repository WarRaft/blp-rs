use crate::BlpError;
use num_enum::TryFromPrimitive;
use std::fmt;

/// A BLP texture's header: what it is and where its parts are. Pixels stay
/// in the buffer it was parsed from; [`Blp::decode`] reads them.
#[derive(Debug, Clone)]
pub struct Blp {
    pub version: Version,
    pub texture_type: TextureType,
    /// BLP2 only: 1 palette, 2 DXT, 3 raw BGRA (see [`Blp::encoding`]).
    pub compression: u8,
    /// Bits of alpha per pixel: 0, 1, 4 or 8.
    pub alpha_bits: u32,
    /// BLP2 only, the DXT variant: 0 DXT1, 1 DXT3, 7 DXT5.
    pub alpha_type: u8,
    /// Size of the base mip, in pixels.
    pub width: u32,
    pub height: u32,
    /// BLP0/BLP1 only: how the game uses the texture (team color, …).
    pub extra: u32,
    /// Whether the texture has mips beyond the base one (flag as written).
    pub has_mipmaps: u32,
    /// Bytes of the file that no part of the texture uses.
    pub holes: usize,
    /// Where the shared data is: the JPEG header for JPEG textures, the
    /// 256-entry BGRA palette (1024 bytes) otherwise.
    pub header: Frame,
}

/// The BLP version, from the file's first four bytes.
#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd, Eq, TryFromPrimitive)]
#[repr(u32)]
pub enum Version {
    /// Warcraft III beta: mips live in separate `.b00`…`.b15` files.
    BLP0 = 0x424C5030,
    /// Warcraft III.
    #[default]
    BLP1 = 0x424C5031,
    /// World of Warcraft.
    BLP2 = 0x424C5032,
}

/// The texture type field.
#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd, Eq, TryFromPrimitive)]
#[repr(u32)]
pub enum TextureType {
    #[default]
    JPEG = 0,
    /// "Direct": a palette in BLP0/BLP1; in BLP2 a palette, DXT or raw
    /// BGRA, as `compression` says.
    PALETTE = 1,
}

/// How the pixels of the mips are stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    /// JPEG (BGRA as CMYK) with a header shared by every mip.
    Jpeg,
    /// One byte per pixel indexing a 256-color palette, then the alpha.
    Palette,
    Dxt1,
    Dxt3,
    Dxt5,
    /// Four bytes per pixel, BGRA.
    Bgra,
}

impl fmt::Display for Encoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Encoding::Jpeg => "JPEG",
            Encoding::Palette => "Palette",
            Encoding::Dxt1 => "DXT1",
            Encoding::Dxt3 => "DXT3",
            Encoding::Dxt5 => "DXT5",
            Encoding::Bgra => "BGRA",
        })
    }
}

/// Where one part of an image is: a mip of a BLP (its size, and its bytes
/// at `offset..offset + length` of the file), a frame of a GIF.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub offset: usize,
    pub length: usize,
}

impl Frame {
    /// The frame's bytes in `buf`; empty for a mip the file doesn't have.
    pub fn data<'a>(&self, buf: &'a [u8]) -> Result<&'a [u8], BlpError> {
        self.offset
            .checked_add(self.length)
            .and_then(|end| buf.get(self.offset..end))
            .ok_or_else(|| {
                BlpError::new("blp.data.oob")
                    .with_arg("offset", self.offset)
                    .with_arg("length", self.length)
                    .with_arg("file", buf.len())
            })
    }
}

impl Blp {
    /// Reads the header of a BLP file: the texture and its mips, the base
    /// one first. Mip `i` is `width >> i` by `height >> i` (at least 1);
    /// one the file doesn't have has `length` 0. Trailing missing mips
    /// aren't listed, the base one always is.
    pub fn parse(buf: &[u8]) -> Result<(Blp, Vec<Frame>), BlpError> {
        crate::blp::parse::parse(buf)
    }

    /// How the mips' pixels are stored.
    pub fn encoding(&self) -> Result<Encoding, BlpError> {
        match (self.texture_type, self.version) {
            (TextureType::JPEG, _) => Ok(Encoding::Jpeg),
            (TextureType::PALETTE, Version::BLP0 | Version::BLP1) => Ok(Encoding::Palette),
            (TextureType::PALETTE, Version::BLP2) => match (self.compression, self.alpha_type) {
                (1, _) => Ok(Encoding::Palette),
                (2, 0) => Ok(Encoding::Dxt1),
                (2, 1) => Ok(Encoding::Dxt3),
                (2, 7) => Ok(Encoding::Dxt5),
                (3, _) => Ok(Encoding::Bgra),
                (compression, alpha_type) => Err(BlpError::new("blp.encoding.unknown")
                    .with_arg("compression", compression)
                    .with_arg("alpha_type", alpha_type)),
            },
        }
    }

    /// The data every mip shares: the JPEG header, or the palette.
    pub fn header_data<'a>(&self, buf: &'a [u8]) -> Result<&'a [u8], BlpError> {
        self.header.data(buf)
    }
}

impl crate::traits::FormatDetector for Blp {
    fn detect(buf: &[u8]) -> bool {
        buf.len() >= 4 && matches!(&buf[..4], b"BLP0" | b"BLP1" | b"BLP2")
    }

    fn parse_header(buf: &[u8]) -> Result<(Self, Vec<Frame>), BlpError> {
        Blp::parse(buf)
    }
}

impl crate::traits::ImageDecoder for Blp {
    fn into_dynamic(buf: &[u8]) -> Result<image::DynamicImage, BlpError> {
        let (blp, mips) = Blp::parse(buf)?;
        Ok(image::DynamicImage::ImageRgba8(blp.decode(&mips[0], buf)?))
    }
}
