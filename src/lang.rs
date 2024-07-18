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
            map.insert(Lang::$lang().extension(), Arc::new(Lang::$lang()));
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
    extension: &'static OsStr,
    tree_sitter_types: &'static [&'static str],
}

impl Lang {
    pub fn from_extension(extension: &OsStr) -> Option<Arc<Self>> {
        EXTENSION_LANGUAGE.get(extension).map(Arc::clone)
    }

    pub fn extension(&self) -> &'static OsStr {
        // TODO: should be a list of extensions
        self.extension
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
            extension: OsStr::new("rs"),
            tree_sitter_types: &["string_content"],
        }
    }

    #[cfg(feature = "lang-cpp")]
    pub fn cpp() -> Self {
        Self {
            language: tree_sitter_cpp::language(),
            extension: OsStr::new("cpp"),
            tree_sitter_types: &["string_content"],
        }
    }

    #[cfg(feature = "lang-c")]
    pub fn c() -> Self {
        Self {
            language: tree_sitter_c::language(),
            extension: OsStr::new("c"),
            tree_sitter_types: &["string_content"],
        }
    }

    #[cfg(feature = "lang-go")]
    pub fn go() -> Self {
        Self {
            language: tree_sitter_go::language(),
            extension: OsStr::new("go"),
            tree_sitter_types: &["interpreted_string_literal"],
        }
    }

    #[cfg(feature = "lang-python")]
    pub fn python() -> Self {
        Self {
            language: tree_sitter_python::language(),
            extension: OsStr::new("py"),
            tree_sitter_types: &["string", "concatenated_string"],
        }
    }

    #[cfg(feature = "lang-toml")]
    pub fn toml() -> Self {
        Self {
            language: tree_sitter_toml_ng::language(),
            extension: OsStr::new("toml"),
            tree_sitter_types: &["string"],
        }
    }

    #[cfg(feature = "lang-yaml")]
    pub fn yaml() -> Self {
        Self {
            language: tree_sitter_yaml::language(),
            extension: OsStr::new("yml"),
            tree_sitter_types: &["string_scalar"],
        }
    }

    #[cfg(feature = "lang-json")]
    pub fn json() -> Self {
        Self {
            language: tree_sitter_json::language(),
            extension: OsStr::new("json"),
            tree_sitter_types: &["string_content"],
        }
    }

    #[cfg(feature = "lang-markdown")]
    pub fn markdown() -> Self {
        Self {
            language: tree_sitter_md::language(),
            extension: OsStr::new("md"),
            tree_sitter_types: &["inline"],
        }
    }
}
