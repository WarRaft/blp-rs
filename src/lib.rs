// `src/_decode` removed — functions have been moved into `src/_from` and `Blp`.
// Keep compatibility via `crate::any_image::decode_to_rgba` and `Blp::decode_image`.
// Encoders moved to `crate::blp`.
// legacy _export module removed - exporters moved into `crate::blp::export` canonical module
// legacy compatibility module removed — use `crate::any_image` or `crate::blp` helpers instead
pub mod error;
// legacy module _mipmap removed during API migration
pub mod any_image;
pub mod blp;
pub mod format_detector;
pub mod gif;
pub mod psd;
pub mod jpg;

// Core types (new)
pub use error::error::BlpError;

// Expose new header-based BLP struct
pub use crate::blp::{Blp, Frame};

// Re-export `image` crate and common image types
pub use image;
pub use image::{DynamicImage, RgbaImage};

// from/* helpers re-exported at crate root
pub use crate::blp::inspect_image_dimensions;
// Canonical loader
pub use crate::blp::open_mipmaps;
// Re-export legacy helper for convenience
pub use crate::any_image::decode_to_rgba;
// High-level AnyImage wrapper
pub use crate::any_image::AnyImage;
pub use crate::any_image::AnyImageData;
// Re-export trait for external detection extension
pub use crate::format_detector::FormatDetector;
// format modules
pub use crate::gif::Gif;
pub use crate::psd::PsdImage;
pub use crate::jpg::Jpg;

// -- encode helpers & options
pub use crate::blp::EncodeOptions;
pub use crate::blp::MipSelection;
pub use crate::blp::RescalePolicy;
pub use crate::blp::encode_mipmaps_to_blp_bytes;
pub use crate::blp::encode_rgba_to_blp_bytes;
pub use crate::blp::encode_with_options;
