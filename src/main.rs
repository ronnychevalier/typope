use std::collections::HashMap;
use std::ffi::OsStr;
use std::ops::Deref;
use std::sync::OnceLock;

use clap::Parser;

use tree_sitter::Language;

use orthotypos::lint::Linter;

mod cli;

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

static EXTENSION_LANGUAGE: Lazy<HashMap<&'static OsStr, Language>> = Lazy::new(|| {
    let mut map = HashMap::new();

    #[cfg(feature = "lang-rust")]
    map.insert(OsStr::new("rs"), tree_sitter_rust::language());
    #[cfg(feature = "lang-cpp")]
    map.insert(OsStr::new("cpp"), tree_sitter_cpp::language());
    #[cfg(feature = "lang-c")]
    map.insert(OsStr::new("c"), tree_sitter_c::language());
    #[cfg(feature = "lang-go")]
    map.insert(OsStr::new("go"), tree_sitter_go::language());
    #[cfg(feature = "lang-python")]
    map.insert(OsStr::new("py"), tree_sitter_python::language());
    #[cfg(feature = "lang-toml")]
    map.insert(OsStr::new("toml"), tree_sitter_toml_ng::language());
    #[cfg(feature = "lang-yaml")]
    map.insert(OsStr::new("yml"), tree_sitter_yaml::language());
    #[cfg(feature = "lang-json")]
    map.insert(OsStr::new("json"), tree_sitter_json::language());
    #[cfg(feature = "lang-markdown")]
    map.insert(OsStr::new("md"), tree_sitter_md::language());

    map
});

fn main() -> anyhow::Result<()> {
    let args = crate::cli::Args::parse();

    let report_handler = args.format().into_error_hook();
    miette::set_hook(report_handler)?;

    let mut typo_found = false;
    for file in args.to_walk() {
        let extension = file.path().extension().unwrap_or_default();
        let Some(language) = EXTENSION_LANGUAGE.get(extension) else {
            continue;
        };

        let source_content = std::fs::read(file.path())?;
        let linter = Linter::new(language, source_content, &file.path().to_string_lossy())?;

        for typo in &linter {
            let typo: miette::Report = typo.into();
            eprintln!("{typo:?}");
            typo_found = true;
        }
    }

    if typo_found {
        std::process::exit(1);
    } else {
        Ok(())
    }
}
