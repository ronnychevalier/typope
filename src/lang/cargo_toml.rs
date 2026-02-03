use serde::Deserialize;
use toml::Spanned;

use super::{Language, LintableNode, Mode, Parsed};

/// Lintable metadata of a Rust package (`Cargo.toml`)
#[derive(Deserialize)]
struct Manifest {
    /// Defines a package
    pub package: Option<Package>,

    /// Defines a workspace
    pub workspace: Option<Workspace>,
}

#[derive(Deserialize)]
struct Package {
    /// A description of the package
    pub description: Spanned<String>,
}

#[derive(Deserialize)]
struct Workspace {
    /// Keys for inheriting in packages
    pub package: Option<Package>,
}

/// Parser for `Cargo.toml` files that only returns relevant lintable strings (e.g., the description field)
struct ParsedManifest {
    manifest: Manifest,
}

impl ParsedManifest {
    pub fn new(text: impl AsRef<[u8]>) -> anyhow::Result<Self> {
        let text = String::from_utf8_lossy(text.as_ref());
        let manifest: Manifest = toml::from_str(text.as_ref())?;

        Ok(Self { manifest })
    }
}

impl Parsed for ParsedManifest {
    fn lintable_nodes<'t>(&'t mut self) -> Box<dyn Iterator<Item = LintableNode<'t>> + 't> {
        Box::new(std::iter::empty())
    }

    fn strings<'t>(
        &'t mut self,
        _source: &'t [u8],
    ) -> Box<dyn Iterator<Item = super::LintableString> + 't> {
        let descriptions = self
            .manifest
            .package
            .iter()
            .chain(self.manifest.workspace.iter().flat_map(|w| &w.package))
            .map(|p| &p.description)
            .filter(|s| s.get_ref().len() >= 3)
            .map(Into::into);

        Box::new(descriptions)
    }
}

impl Language {
    /// Creates a language parser for `Cargo.toml` files
    pub fn cargo_toml() -> Self {
        Self {
            name: "Cargo.toml",
            detections: &["Cargo.toml"],
            parser: Mode::Custom(Box::new(move |text| {
                Ok(Box::new(ParsedManifest::new(text)?))
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use crate::SharedSource;
    use crate::lang::LintableString;

    use super::Language;

    #[test]
    fn exists_in_iter() {
        assert!(Language::iter().any(|lang| lang.name() == "Cargo.toml"));
    }

    #[test]
    fn find_from_filename() {
        assert_eq!(
            "Cargo.toml",
            Language::from_filename(OsStr::new("Cargo.toml"))
                .unwrap()
                .name()
        );
    }

    #[test]
    fn lintable_strings() {
        let cargo_toml = include_str!("../../Cargo.toml");
        let toml = SharedSource::new("Cargo.toml", cargo_toml.as_bytes().to_vec());
        let mut parsed = Language::cargo_toml().parse(&toml).unwrap();
        let strings = parsed.strings(toml.as_ref()).collect::<Vec<_>>();
        assert_eq!(
            strings,
            [
                LintableString {
                    offset: 58,
                    value: "Pedantic source code checker for orthotypography mistakes and other typographical errors".into()
                }
            ]
        );
    }
}
