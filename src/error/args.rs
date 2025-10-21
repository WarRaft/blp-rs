use fluent_templates::fluent_bundle::FluentValue;
use std::{borrow::Cow, fmt, fmt::Display, sync::Arc};

#[derive(Clone)]
pub enum ArgVal {
    Str(Arc<str>),
    Int(i64),
    F64(f64),
    Bool(bool),
    Display(Arc<dyn Display + Send + Sync>),
}

impl From<String> for ArgVal {
    #[inline]
    fn from(s: String) -> Self {
        ArgVal::Str(Arc::<str>::from(s))
    }
}

impl From<&String> for ArgVal {
    #[inline]
    fn from(s: &String) -> Self {
        ArgVal::Str(Arc::<str>::from(s.as_str()))
    }
}

impl From<&str> for ArgVal {
    #[inline]
    fn from(s: &str) -> Self {
        ArgVal::Str(Arc::<str>::from(s))
    }
}

impl From<Arc<str>> for ArgVal {
    #[inline]
    fn from(s: Arc<str>) -> Self {
        ArgVal::Str(s)
    }
}

impl From<u32> for ArgVal {
    fn from(v: u32) -> Self {
        ArgVal::Int(v as i64)
    }
}

impl From<u64> for ArgVal {
    fn from(v: u64) -> Self {
        ArgVal::Int(v as i64)
    }
}

impl From<usize> for ArgVal {
    fn from(v: usize) -> Self {
        ArgVal::Int(v as i64)
    }
}

// Debug вручную (dyn Display не Debug)
impl fmt::Debug for ArgVal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArgVal::Str(s) => f.debug_tuple("Str").field(&s).finish(),
            ArgVal::Int(i) => f.debug_tuple("Int").field(i).finish(),
            ArgVal::F64(x) => f.debug_tuple("F64").field(x).finish(),
            ArgVal::Bool(b) => f.debug_tuple("Bool").field(b).finish(),
            ArgVal::Display(d) => {
                let s = format!("{d}");
                f.debug_tuple("Display")
                    .field(&s)
                    .finish()
            }
        }
    }
}

impl ArgVal {
    #[inline]
    pub fn from_display<T: Display + Send + Sync + 'static>(t: T) -> Self {
        ArgVal::Display(Arc::new(t))
    }

    /// Возвращает владел. FluentValue<'static>, безопасный для FluentArgs.
    pub fn to_fluent_owned(&self) -> FluentValue<'static> {
        match self {
            ArgVal::Str(s) => FluentValue::String(Cow::Owned(s.to_string())),
            ArgVal::Int(i) => FluentValue::from(i), // <-- &i64
            ArgVal::F64(f) => FluentValue::from(f), // <-- &f64
            ArgVal::Bool(b) => {
                let lit: &'static str = if *b { "true" } else { "false" };
                FluentValue::String(Cow::Borrowed(lit))
            }
            ArgVal::Display(d) => FluentValue::String(Cow::Owned(format!("{d}"))),
        }
    }
}
