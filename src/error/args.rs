use std::{fmt, sync::Arc};

/// A named argument of a [`BlpError`](crate::BlpError).
#[derive(Debug, Clone)]
pub(crate) enum Arg {
    Str(Arc<str>),
    Int(i64),
}

impl fmt::Display for Arg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Arg::Str(s) => write!(f, "{s:?}"),
            Arg::Int(i) => write!(f, "{i}"),
        }
    }
}

impl From<String> for Arg {
    fn from(s: String) -> Self {
        Arg::Str(s.into())
    }
}

impl From<&str> for Arg {
    fn from(s: &str) -> Self {
        Arg::Str(s.into())
    }
}

macro_rules! int_arg {
    ($($t:ty),*) => {$(
        impl From<$t> for Arg {
            fn from(v: $t) -> Self {
                Arg::Int(v as i64)
            }
        }
    )*};
}

int_arg!(u8, u32, u64, usize);
