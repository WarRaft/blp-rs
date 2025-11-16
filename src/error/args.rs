use std::fmt::Debug;
use std::{fmt, fmt::Display, sync::Arc};

#[derive(Clone)]
pub(crate) enum Arg {
    Str(Arc<str>),
    Int(i64),
    F64(f64),
    Bool(bool),
    Display(Arc<dyn Display + Send + Sync>),
}

impl Debug for Arg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Arg::Str(s) => f.debug_tuple("Str").field(&s).finish(),
            Arg::Int(i) => f.debug_tuple("Int").field(i).finish(),
            Arg::F64(x) => f.debug_tuple("F64").field(x).finish(),
            Arg::Bool(b) => f.debug_tuple("Bool").field(b).finish(),
            Arg::Display(d) => {
                let s = format!("{d}");
                f.debug_tuple("Display")
                    .field(&s)
                    .finish()
            }
        }
    }
}

impl From<String> for Arg {
    #[inline]
    fn from(s: String) -> Self {
        Arg::Str(Arc::<str>::from(s))
    }
}

impl From<&String> for Arg {
    #[inline]
    fn from(s: &String) -> Self {
        Arg::Str(Arc::<str>::from(s.as_str()))
    }
}

impl From<&str> for Arg {
    #[inline]
    fn from(s: &str) -> Self {
        Arg::Str(Arc::<str>::from(s))
    }
}

impl From<Arc<str>> for Arg {
    #[inline]
    fn from(s: Arc<str>) -> Self {
        Arg::Str(s)
    }
}

impl From<u32> for Arg {
    fn from(v: u32) -> Self {
        Arg::Int(v as i64)
    }
}

impl From<u64> for Arg {
    fn from(v: u64) -> Self {
        Arg::Int(v as i64)
    }
}

impl From<usize> for Arg {
    fn from(v: usize) -> Self {
        Arg::Int(v as i64)
    }
}
