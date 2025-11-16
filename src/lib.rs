pub mod any_image;
pub mod blp;
pub mod error;
pub mod traits;

// Internal format modules
pub(crate) mod gif;
pub(crate) mod jpg;
pub(crate) mod psd;

// Core types
pub use error::error::BlpError;

// BLP structures
pub use crate::blp::{Blp, Frame, TextureType, Version};

// Re-export `image` crate and common image types
pub use image;
pub use image::{DynamicImage, RgbaImage};

// High-level AnyImage wrapper
pub use crate::any_image::AnyImage;
pub use crate::any_image::AnyImageData;
pub use crate::any_image::EncodeMipOptions;
pub use crate::any_image::EncodeOptions as AnyImageEncodeOptions;

// Re-export traits for external detection extension
pub use crate::traits::FormatDetector;
pub use crate::traits::ImageDecoder;

// BLP encode helpers & options
pub use crate::blp::EncodeOptions;
pub use crate::blp::MipSelection;
pub use crate::blp::RescalePolicy;
