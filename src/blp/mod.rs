//! The BLP format: [`Blp::parse`] reads the header, [`Blp::decode`] a mip,
//! [`Blp::encode_from_rgba`] (feature `encode`) writes a BLP1 JPEG
//! texture.

#[allow(clippy::module_inception)]
mod blp;
mod decode;
#[cfg(feature = "encode")]
mod encode;
mod parse;

pub use blp::{Blp, Encoding, Frame, TextureType, Version};

/// Mip slots in a BLP1/BLP2 header.
pub(crate) const MAX_MIPS: usize = 16;
