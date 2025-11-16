pub mod any_image;
pub mod blp;
pub mod error;
pub mod format_detector;
pub mod gif;
pub mod jpg;
pub mod psd;

// Core types
pub use error::error::BlpError;

// BLP structures
pub use crate::blp::{Blp, Frame};

// Re-export `image` crate and common image types
pub use image;
pub use image::{DynamicImage, RgbaImage};

// from/* helpers re-exported at crate root
pub use crate::blp::inspect_image_dimensions;
pub use crate::blp::open_mipmaps;
pub use crate::any_image::decode_to_rgba;
// High-level AnyImage wrapper
pub use crate::any_image::AnyImage;
pub use crate::any_image::AnyImageData;
pub use crate::any_image::EncodeMipOptions;
pub use crate::any_image::EncodeOptions as AnyImageEncodeOptions;
// Re-export trait for external detection extension
pub use crate::format_detector::FormatDetector;
pub use crate::format_detector::ImageDecoder;
// format modules
pub use crate::gif::Gif;
pub use crate::jpg::Jpg;
pub use crate::psd::PsdImage;

// -- encode helpers & options
pub use crate::blp::EncodeOptions;
pub use crate::blp::MipSelection;
pub use crate::blp::RescalePolicy;
