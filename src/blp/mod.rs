pub mod blp;
pub use blp::{Blp, Frame, TextureType, Version};

// Export constants for internal use
pub(crate) use blp::{MAX_MIPS, HEADER_SIZE};

// Internal modules - not part of public API
pub(crate) mod parse;
pub(crate) use parse::{parse_header, header_data, mip_raw};

pub(crate) mod decode;
pub(crate) mod encode;

pub mod options;
pub use options::{EncodeOptions, MipSelection, RescalePolicy};
