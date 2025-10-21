use crate::error::args::ArgVal;
use crate::error::cause::Cause;
use num_enum::TryFromPrimitiveError;
use std::{collections::BTreeMap, fmt, io, sync::Arc};

#[derive(Debug, Clone)]
pub struct BlpError {
    pub key: &'static str,
    pub args: BTreeMap<&'static str, ArgVal>,
    pub causes: Vec<Cause>,
}

impl BlpError {
    #[inline]
    pub fn ctx(self, key: &'static str) -> BlpError {
        BlpError::new(key).push_blp(self)
    }

    #[inline]
    pub fn ctx_with(self, key: &'static str, f: impl FnOnce(BlpError) -> BlpError) -> BlpError {
        f(BlpError::new(key).push_blp(self))
    }

    #[inline]
    pub fn new(key: &'static str) -> Self {
        Self { key, args: BTreeMap::new(), causes: Vec::new() }
    }

    #[inline]
    pub fn with_arg(mut self, name: &'static str, val: impl Into<ArgVal>) -> Self {
        self.args.insert(name, val.into());
        self
    }

    #[inline]
    pub fn with_args(mut self, args: impl IntoIterator<Item = (&'static str, ArgVal)>) -> Self {
        for (k, v) in args {
            self.args.insert(k, v);
        }
        self
    }

    #[inline]
    pub fn push_blp(mut self, cause: BlpError) -> Self {
        self.causes.push(Cause::Blp(cause));
        self
    }

    #[inline]
    pub fn push_std(mut self, cause: impl std::error::Error + Send + Sync + 'static) -> Self {
        self.causes
            .push(Cause::Std(Arc::new(cause)));
        self
    }
}

impl fmt::Display for BlpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}(", self.key)?;
        let mut first = true;
        for (k, v) in &self.args {
            if !first {
                write!(f, ", ")?;
            }
            first = false;
            write!(f, "{k}={v:?}")?;
        }
        write!(f, ")")
    }
}

impl std::error::Error for BlpError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.causes
            .iter()
            .find_map(|c| match c {
                Cause::Blp(e) => Some(e as &dyn std::error::Error),
                Cause::Std(e) => Some(e.as_ref()),
            })
    }
}

impl From<io::Error> for BlpError {
    fn from(e: io::Error) -> Self {
        BlpError::new("io-error").push_std(e)
    }
}

impl From<image::ImageError> for BlpError {
    fn from(e: image::ImageError) -> Self {
        BlpError::new("image-error").push_std(e)
    }
}

impl From<image::error::DecodingError> for BlpError {
    fn from(e: image::error::DecodingError) -> Self {
        BlpError::new("png-error").push_std(e)
    }
}

impl From<jpeg_decoder::Error> for BlpError {
    fn from(e: jpeg_decoder::Error) -> Self {
        BlpError::new("jpeg-error").push_std(e)
    }
}

impl From<turbojpeg::Error> for BlpError {
    fn from(e: turbojpeg::Error) -> Self {
        BlpError::new("turbojpeg-error").push_std(e)
    }
}

impl<T> From<TryFromPrimitiveError<T>> for BlpError
where
    T: num_enum::TryFromPrimitive + 'static,
    T::Primitive: Copy + Into<u64>,
{
    fn from(_err: TryFromPrimitiveError<T>) -> Self {
        BlpError::new("num-error").with_arg("name", core::any::type_name::<T>())
    }
}
