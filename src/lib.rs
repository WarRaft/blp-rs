pub mod _blp_image;
pub mod _decode;
mod _encode;
pub mod _error;
mod _export;
mod _from;
pub mod _mipmap;
pub mod _types;
mod blp_image_decoder;
mod blp_image_encoder;

pub use crate::blp_image_decoder::BlpImageDecoder;
pub use crate::blp_image_encoder::BlpImageEncoder;

// Core types
pub use _blp_image::BlpImage;
pub use _blp_image::{BlpMeta, MAX_MIPS, MipInfo};
pub use _error::error::BlpError;

// Re-export commonly used enums from types at crate root for convenience in examples
pub use _types::SourceKind;
pub use _types::TextureType;
pub use _types::Version;

// Re-export `image` crate and common image types so examples and consumers
// can use `blp::image::DynamicImage` / `blp::DynamicImage` without
// depending on `image` directly.
pub use image;
pub use image::{DynamicImage, RgbaImage};

// -- from/* helpers re-exported at crate root
// `from` module already re-exports these helpers; re-export them at crate root.
pub use crate::_from::{inspect_image_dimensions, load_image_dynamic, parse_blp_image, parse_blp_meta};

// -- decode helpers
pub use crate::_decode::decode_to_rgba;
// Internal decode implementations (exposed for advanced usage)
pub use crate::_decode::{decode_direct_to_mipmaps, decode_image_to_mipmaps, decode_jpeg_to_mipmaps};

// -- encode helpers & options
pub use crate::_encode::EncodeOptions;
pub use crate::_encode::MipSelection;
pub use crate::_encode::RescalePolicy;
pub use crate::_encode::encode_mipmaps_to_blp_bytes;
pub use crate::_encode::encode_rgba_to_blp_bytes;
pub use crate::_encode::encode_with_options;

// -- export helpers
pub use crate::_export::export_blp_to_path;
