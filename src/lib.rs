pub mod _decode;
mod _encode;
pub mod _error;
mod _export;
mod _from;
// legacy module _mipmap removed during API migration
pub mod any_image;
pub mod blp;
pub mod gif;
pub mod psd;

// Core types (new)
pub use _error::error::BlpError;

// Expose new header-based BLP struct
pub use crate::blp::{Blp, Frame};

// Re-export `image` crate and common image types
pub use image;
pub use image::{DynamicImage, RgbaImage};

// from/* helpers re-exported at crate root
pub use crate::_from::{inspect_image_dimensions, load_image_dynamic, parse_blp_image, parse_blp_meta};
// Canonical loader
pub use crate::_from::open;
pub use crate::_from::open_mipmaps;
// High-level AnyImage wrapper
pub use crate::any_image::AnyImage;
// format modules
pub use crate::gif::Gif;
pub use crate::psd::PsdImage;

// -- decode helpers
pub use crate::_decode::decode_to_rgba;
pub use crate::_decode::{decode_direct_to_mipmaps, decode_image_to_mipmaps, decode_jpeg_to_mipmaps};

// -- encode helpers & options
pub use crate::_encode::EncodeOptions;
pub use crate::_encode::MipSelection;
pub use crate::_encode::RescalePolicy;
pub use crate::_encode::encode_mipmaps_to_blp_bytes;
pub use crate::_encode::encode_rgba_to_blp_bytes;
pub use crate::_encode::encode_with_options;
