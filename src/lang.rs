use std::collections::HashMap;
use std::ffi::OsStr;
use std::ops::Deref;
use std::sync::{Arc, OnceLock};

use tree_sitter::Language;

struct Lazy<T> {
    cell: OnceLock<T>,
    init: fn() -> T,
}

impl<T> Lazy<T> {
    pub const fn new(init: fn() -> T) -> Self {
        Self {
            cell: OnceLock::new(),
            init,
        }
    }
}

impl<T> Deref for Lazy<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &'_ T {
        self.cell.get_or_init(self.init)
    }
}

static EXTENSION_LANGUAGE: Lazy<HashMap<&'static OsStr, Arc<Lang>>> = Lazy::new(|| {
    let mut map = HashMap::new();

    macro_rules! lang {
        ($lang:ident, $feature: literal) => {
            #[cfg(feature = $feature)]
            {
                let lang = Arc::new(Lang::$lang());
                for extension in lang.extensions() {
                    map.insert(OsStr::new(extension), Arc::clone(&lang));
                }
            }
        };
    }

    lang!(rust, "lang-rust");
    lang!(c, "lang-c");
    lang!(cpp, "lang-cpp");
    lang!(go, "lang-go");
    lang!(python, "lang-python");
    lang!(toml, "lang-toml");
    lang!(yaml, "lang-yaml");
    lang!(json, "lang-json");
    lang!(markdown, "lang-markdown");

    map
});

pub struct Lang {
    language: Language,
    extensions: &'static [&'static str],
    tree_sitter_types: &'static [&'static str],
}

impl Lang {
    pub fn from_extension(extension: &OsStr) -> Option<Arc<Self>> {
        EXTENSION_LANGUAGE.get(extension).map(Arc::clone)
    }

    pub fn extensions(&self) -> &'static [&'static str] {
        self.extensions
    }

    pub fn language(&self) -> &Language {
        &self.language
    }

    /// Returns the tree-sitter types that should be parsed
    pub fn tree_sitter_types(&self) -> &[&str] {
        self.tree_sitter_types
    }

    #[cfg(feature = "lang-rust")]
    pub fn rust() -> Self {
        Self {
            language: tree_sitter_rust::language(),
            extensions: &["rs"],
            tree_sitter_types: &["string_content"],
        }
    }

    #[cfg(feature = "lang-cpp")]
    pub fn cpp() -> Self {
        Self {
            language: tree_sitter_cpp::language(),
            extensions: &["cpp", "cc", "hpp", "hh"],
            tree_sitter_types: &["string_content"],
        }
    }

    #[cfg(feature = "lang-c")]
    pub fn c() -> Self {
        Self {
            language: tree_sitter_c::language(),
            extensions: &["c", "h"],
            tree_sitter_types: &["string_content"],
        }
    }

    #[cfg(feature = "lang-go")]
    pub fn go() -> Self {
        Self {
            language: tree_sitter_go::language(),
            extensions: &["go"],
            tree_sitter_types: &["interpreted_string_literal"],
        }
    }

    #[cfg(feature = "lang-python")]
    pub fn python() -> Self {
        Self {
            language: tree_sitter_python::language(),
            extensions: &["py"],
            tree_sitter_types: &["string", "concatenated_string"],
        }
    }

    #[cfg(feature = "lang-toml")]
    pub fn toml() -> Self {
        Self {
            language: tree_sitter_toml_ng::language(),
            extensions: &["toml"],
            tree_sitter_types: &["string"],
        }
    }

    #[cfg(feature = "lang-yaml")]
    pub fn yaml() -> Self {
        Self {
            language: tree_sitter_yaml::language(),
            extensions: &["yml", "yaml"],
            tree_sitter_types: &["string_scalar"],
        }
    }

    #[cfg(feature = "lang-json")]
    pub fn json() -> Self {
        Self {
            language: tree_sitter_json::language(),
            extensions: &["json"],
            tree_sitter_types: &["string_content"],
        }
    }

    #[cfg(feature = "lang-markdown")]
    pub fn markdown() -> Self {
        Self {
            language: tree_sitter_md::language(),
            extensions: &["md"],
            tree_sitter_types: &["inline"],
        }
    }
}
