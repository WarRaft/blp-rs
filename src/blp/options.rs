use crate::error::error::BlpError;

/// How to choose which mip levels to produce.
#[derive(Debug, Clone)]
pub enum MipSelection {
    /// Use encoder defaults (use all available mips / existing images).
    Auto,
    /// Produce exactly `n` mip levels (base + n-1 lower levels).
    Count(usize),
    /// Explicit boolean mask per mip level (true = produce).
    Explicit(Vec<bool>),
}

/// Policy for handling non-power-of-two sources.
#[derive(Debug, Clone)]
pub enum RescalePolicy {
    /// Automatically resize to power-of-two cover during encoding (default behavior).
    Auto,
    /// Fail if sizes don't match power-of-two expectations.
    Error,
    /// Crop or pad to the target frame (not implemented yet).
    Crop,
}

/// Encode options for BLP encoding.
#[derive(Debug, Clone)]
pub struct EncodeOptions {
    /// JPEG quality 0..100
    pub quality: u8,
    /// How to select mip levels
    pub mip: MipSelection,
    /// Rescale policy for non-power-of-two sources
    pub rescale: RescalePolicy,
}

impl Default for EncodeOptions {
    fn default() -> Self {
        EncodeOptions { quality: 90, mip: MipSelection::Auto, rescale: RescalePolicy::Auto }
    }
}

impl EncodeOptions {
    pub fn validate(&self) -> Result<(), BlpError> {
        if self.quality > 100 { return Err(BlpError::new("encode.invalid_quality").with_arg("quality", self.quality as u32)); }
        Ok(())
    }
}
