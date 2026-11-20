//! Blizzard's BLP textures (Warcraft III, World of Warcraft): read every
//! kind (BLP1/BLP2; JPEG, palette, DXT1/3/5, raw BGRA), write BLP1 JPEG.
//!
//! [`Blp::parse`] reads the header and where each mip lives; [`Blp::decode`]
//! turns one mip into RGBA. [`AnyImage`] wraps any supported input (BLP,
//! PNG, JPEG, GIF, PSD, …) behind one API that decodes and converts.
//!
//! ```no_run
//! use blp::Blp;
//!
//! let buf = std::fs::read("texture.blp")?;
//! let (blp, mips) = Blp::parse(&buf)?;
//! println!("{} {}x{}, {} mips", blp.encoding()?, blp.width, blp.height, mips.len());
//! let base = blp.decode(&mips[0], &buf)?;
//! base.save("texture.png")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Features (all on by default): `encode` (writing BLP, needs a C
//! toolchain for libjpeg-turbo), `psd` (Photoshop input), `formats` (every
//! format the `image` crate reads). With none of them the crate is pure
//! Rust and still decodes BLP and exports PNG/JPEG.

pub mod any_image;
pub mod blp;
pub mod error;
pub mod traits;

mod gif;
mod jpg;
#[cfg(feature = "psd")]
mod psd;

pub use crate::any_image::{AnyImage, AnyImageData, EncodeMipOptions, EncodeOptions};
pub use crate::blp::{Blp, Encoding, Frame, TextureType, Version};
pub use crate::error::BlpError;
pub use crate::traits::{FormatDetector, ImageDecoder};
pub use image::{self, DynamicImage, RgbaImage};
