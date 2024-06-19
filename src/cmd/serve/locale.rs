use std::sync::Arc;

use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, HeaderMap},
};
use camino::Utf8Path;
use minijinja::{value::Object, Value};

use super::{get_cookie, Locales};

pub struct LocaleStore {
    pub locales: Vec<Arc<Locale>>,
}

impl LocaleStore {
    const DEFAULT_LOCALE: &'static str = "en";

    pub fn get(&self, code: &str) -> Option<Arc<Locale>> {
        self.locales
            .iter()
            .find(|l| l.code.starts_with(code))
            .cloned()
    }

    pub fn get_default(&self) -> Arc<Locale> {
        self.get(Self::DEFAULT_LOCALE)
            .expect("can't load default locale")
    }

    pub fn get_from_cookie(&self, headers: &HeaderMap) -> Option<Arc<Locale>> {
        let value = get_cookie(headers, "language")?;
        self.get(value)
    }

    pub fn get_from_langs(&self, headers: &HeaderMap) -> Option<Arc<Locale>> {
        let langs = headers
            .get(axum::http::header::ACCEPT_LANGUAGE)
            .and_then(|e| e.to_str().ok())?;
        langs.split(',').find_map(|code| self.get(code))
    }

    pub fn get_from_headers(&self, headers: &HeaderMap) -> Arc<Locale> {
        self.get_from_cookie(headers)
            .or_else(|| self.get_from_langs(headers))
            .unwrap_or_else(|| self.get_default())
    }
}

#[derive(Debug)]
pub struct Locale {
    pub code: String,
    pub translations: serde_json::Value,
}

impl Locale {
    pub fn get(&self, key: &str) -> Option<&str> {
        let mut current = &self.translations;
        for part in key.split('.') {
            current = current.as_object()?.get(part)?;
        }
        let value = current.as_str()?;
        Some(value)
    }
}

impl Object for Locale {
    fn call(
        self: &Arc<Self>,
        state: &minijinja::State,
        args: &[minijinja::Value],
    ) -> Result<minijinja::Value, minijinja::Error> {
        if args.is_empty() {
            return Err(minijinja::Error::new(
                minijinja::ErrorKind::MissingArgument,
                "missing 'key' arg for translation",
            ));
        }
        if args.len() > 2 {
            return Err(minijinja::Error::new(
                minijinja::ErrorKind::TooManyArguments,
                "first arg 'key', second arg (optional) context",
            ));
        }
        let key = args[0].as_str().ok_or(minijinja::Error::new(
            minijinja::ErrorKind::InvalidOperation,
            "expected str for translation",
        ))?;

        let val = match self.get(key) {
            Some(t) => t,
            None => {
                #[cfg(debug_assertions)]
                {
                    return Err(minijinja::Error::new(
                        minijinja::ErrorKind::InvalidOperation,
                        "translation not found",
                    ));
                }
                #[cfg(not(debug_assertions))]
                ""
            }
        };

        let val = if args.len() == 2 {
            match state.env().render_str(val, &args[1]) {
                Ok(val) => val,
                Err(e) => {
                    return Err(minijinja::Error::new(
                        minijinja::ErrorKind::InvalidOperation,
                        "failed to render translation template",
                    )
                    .with_source(e))
                }
            }
        } else {
            val.to_string()
        };
        Ok(Value::from(val))
    }

    fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
        match key.as_str() {
            Some("code") => Some(self.code.as_str().into()),
            _ => None,
        }
    }

    fn enumerate(self: &Arc<Self>) -> minijinja::value::Enumerator {
        minijinja::value::Enumerator::Str(&["code"])
    }
}

pub fn make_locale_store() -> LocaleStore {
    let mut locales = Vec::new();
    for locale in Locales::iter() {
        let content = Locales::get(&locale).unwrap();
        let translations: serde_json::Value =
            serde_json::from_slice(&content.data).expect("can't parse translations");
        let code = Utf8Path::new(&locale)
            .file_stem()
            .expect("bad locale file name")
            .to_string();
        locales.push(Arc::new(Locale { code, translations }));
    }
    LocaleStore { locales }
}

pub struct UserLocale(pub minijinja::Value);

#[async_trait]
impl FromRequestParts<super::S> for UserLocale {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &super::S,
    ) -> Result<Self, Self::Rejection> {
        Ok(Self(Value::from_dyn_object(
            state.locales.get_from_headers(&parts.headers),
        )))
    }
}
