use crate::ui::viewer::app::App;
use fluent_templates::fluent_bundle::{FluentArgs, FluentValue};
use fluent_templates::{Loader, static_loader};
use std::borrow::Cow;
use std::collections::HashMap;

// Сгенерированный при компиляции лоадер.
// Он сам найдёт assets/locales/{lang}/*.ftl, вшьёт и даст API lookup/lookup_with_args.
static_loader! {
    pub static LOCALES = {
        locales: "./assets/locales",
        fallback_language: "en",
        // можно тонко настроить бандл:
        customise: |bundle| bundle.set_use_isolating(false),
    };
}

// Шорткаты перевода
impl App {
    #[inline]
    pub fn tr(&self, key: &str) -> String {
        LOCALES.lookup(&self.lng.id(), key)
    }

    #[inline]
    pub fn tr_args(&self, key: &str, args: &FluentArgs) -> String {
        // FluentArgs -> HashMap<Cow<'static, str>, FluentValue>
        let map: HashMap<Cow<'static, str>, FluentValue> = args
            .iter()
            .map(|(k, v)| (Cow::Owned(k.to_string()), v.clone()))
            .collect();

        LOCALES.lookup_with_args(&self.lng.id(), key, &map)
    }
}

// 1) FluentArgs
#[macro_export]
macro_rules! flargs {
    ($($k:ident = $v:expr),* $(,)?) => {{
        let mut __a = fluent_templates::fluent_bundle::FluentArgs::new();
        $(
            __a.set(stringify!($k),
                    fluent_templates::fluent_bundle::FluentValue::from($v));
        )*
        __a
    }};
}

// 2) HashMap<Cow<'static, str>, FluentValue>
#[macro_export]
macro_rules! flmap {
    ($($k:ident = $v:expr),* $(,)?) => {{
        let mut __m: ::std::collections::HashMap<
            ::std::borrow::Cow<'static, str>,
            fluent_templates::fluent_bundle::FluentValue
        > = ::std::collections::HashMap::new();
        $(
            __m.insert(::std::borrow::Cow::Owned(stringify!($k).to_string()),
                       fluent_templates::fluent_bundle::FluentValue::from($v));
        )*
        __m
    }};
}
