//! Idiomas: cada texto del contenido viaja como `{ en, es }` en el mismo archivo.

use serde::Deserialize;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Lang {
    #[default]
    En,
    Es,
}

impl Lang {
    pub fn toggle(self) -> Self {
        match self {
            Lang::En => Lang::Es,
            Lang::Es => Lang::En,
        }
    }
}

/// Texto de la interfaz que no vive en el contenido.
pub fn tr(lang: Lang, en: &'static str, es: &'static str) -> &'static str {
    match lang {
        Lang::En => en,
        Lang::Es => es,
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Text {
    pub en: String,
    pub es: String,
}

impl Text {
    pub fn get(&self, lang: Lang) -> &str {
        match lang {
            Lang::En => &self.en,
            Lang::Es => &self.es,
        }
    }

    pub fn is_complete(&self) -> bool {
        !self.en.trim().is_empty() && !self.es.trim().is_empty()
    }
}
