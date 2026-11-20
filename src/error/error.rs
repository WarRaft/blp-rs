use crate::error::{args::Arg, cause::Cause};
use num_enum::TryFromPrimitiveError;
use std::{collections::BTreeMap, fmt, io, sync::Arc};

/// An error: a stable key (`"blp.mip.oob"`), named arguments, and the
/// errors that caused it.
#[derive(Debug, Clone)]
pub struct BlpError {
    pub key: &'static str,
    pub(crate) args: BTreeMap<&'static str, Arg>,
    pub(crate) causes: Vec<Cause>,
}

impl BlpError {
    #[inline]
    pub fn new(key: &'static str) -> Self {
        Self { key, args: BTreeMap::new(), causes: Vec::new() }
    }

    /// Wraps this error as the cause of a new one.
    #[inline]
    pub fn ctx(self, key: &'static str) -> BlpError {
        BlpError::new(key).push_blp(self)
    }

    #[inline]
    pub(crate) fn with_arg(mut self, name: &'static str, val: impl Into<Arg>) -> Self {
        self.args.insert(name, val.into());
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

/// `key(name=value, …): cause: cause…`
impl fmt::Display for BlpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.key)?;
        if !self.args.is_empty() {
            f.write_str("(")?;
            for (i, (k, v)) in self.args.iter().enumerate() {
                if i > 0 {
                    f.write_str(", ")?;
                }
                write!(f, "{k}={v}")?;
            }
            f.write_str(")")?;
        }
        for cause in &self.causes {
            match cause {
                Cause::Blp(e) => write!(f, ": {e}")?,
                Cause::Std(e) => write!(f, ": {e}")?,
            }
        }
        Ok(())
    }
}

impl std::error::Error for BlpError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.causes.first().map(|c| match c {
            Cause::Blp(e) => e as &dyn std::error::Error,
            Cause::Std(e) => e.as_ref() as &dyn std::error::Error,
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

impl From<jpeg_decoder::Error> for BlpError {
    fn from(e: jpeg_decoder::Error) -> Self {
        BlpError::new("jpeg-error").push_std(e)
    }
}

impl<T> From<TryFromPrimitiveError<T>> for BlpError
where
    T: num_enum::TryFromPrimitive + 'static,
    T::Primitive: Copy + Into<u64>,
{
    fn from(err: TryFromPrimitiveError<T>) -> Self {
        let name = core::any::type_name::<T>();
        BlpError::new("num-error")
            .with_arg("type", name.rsplit("::").next().unwrap_or(name))
            .with_arg("value", err.number.into())
    }
}
