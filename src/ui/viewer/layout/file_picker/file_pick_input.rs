use crate::core::image::ImageBlp;
use crate::error::error::BlpError;
use crate::ext::path::ensure_readable::EnsureReadable;
use std::path::PathBuf;

pub enum FilePickInput {
    Path(PathBuf),
    Bytes(Vec<u8>),
}

impl FilePickInput {
    fn into_bytes(self) -> Result<Vec<u8>, BlpError> {
        match self {
            FilePickInput::Path(path) => path.as_path().read_all(),
            FilePickInput::Bytes(data) => Ok(data),
        }
    }

    pub fn decode(self) -> Result<ImageBlp, BlpError> {
        let data = self.into_bytes()?;
        let mut img = ImageBlp::from_buf(&data)?;
        img.decode(&data, &[])?;
        Ok(img)
    }
}
