use num_enum::TryFromPrimitive;
pub struct Blp {
    pub version: Version,
    pub texture_type: TextureType,
    pub compression: u8,
    pub alpha_bits: u32,
    pub alpha_type: u8,
    pub has_mips: u8,
    pub width: u32,
    pub height: u32,
    pub extra: u32,       // meaningful only if version <= BLP1
    pub has_mipmaps: u32, // meaningful only if version <= BLP1 or >= BLP2
    pub frames: Vec<Frame>,
    pub holes: usize,
    pub header: Frame,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd, Eq, TryFromPrimitive)]
#[repr(u32)]
pub enum Version {
    BLP0 = 0x424C5030, // "BLP0"
    #[default]
    BLP1 = 0x424C5031, // "BLP1"
    BLP2 = 0x424C5032, // "BLP2"
}

#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd, Eq, TryFromPrimitive)]
#[repr(u32)]
pub enum TextureType {
    #[default]
    JPEG = 0,
    PALETTE = 1,
}

#[derive(Debug, Default)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub offset: usize,
    pub length: usize,
}
