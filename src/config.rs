//! Config parsers to recognize the config fields of [`typos`](https://crates.io/crates/typos-cli).
// It is based on <https://github.com/crate-ci/typos/blob/master/crates/typos-cli/src/config.rs>
// but it has been modified to remove fields that we do not care about for the moment.
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Context;

use ignore::WalkBuilder;

use crate::lang::Language;

/// List of file names that can contain the configuration
pub const SUPPORTED_FILE_NAMES: &[&str] = &[
    "typos.toml",
    "_typos.toml",
    ".typos.toml",
    CARGO_TOML,
    PYPROJECT_TOML,
];
const CARGO_TOML: &str = "Cargo.toml";
const PYPROJECT_TOML: &str = "pyproject.toml";

/// Defines the configuration of the linter.
///
/// It is compatible with a subset of the configuration of [`typos`](https://crates.io/crates/typos-cli).
///
/// # Example
///
/// ```toml
/// [files]
/// ignore-hidden = false
///
/// [default]
/// extend-ignore-re = ["some regex.*rrrregex"]
///
/// [type.cpp]
/// check-file = false
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    pub files: Walk,
    pub default: EngineConfig,
    #[serde(rename = "type")]
    pub type_: TypeEngineConfig,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[serde(rename_all = "kebab-case")]
struct CargoTomlConfig {
    pub workspace: Option<CargoTomlPackage>,
    pub package: Option<CargoTomlPackage>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[serde(rename_all = "kebab-case")]
struct CargoTomlPackage {
    pub metadata: CargoTomlMetadata,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[serde(rename_all = "kebab-case")]
struct CargoTomlMetadata {
    pub typope: Option<Config>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[serde(rename_all = "kebab-case")]
struct PyprojectTomlConfig {
    tool: PyprojectTomlTool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[serde(rename_all = "kebab-case")]
struct PyprojectTomlTool {
    typos: Option<Config>,
}

impl Config {
    /// Tries to load a config from a directory.
    ///
    /// It looks for the file names listed in [`SUPPORTED_FILE_NAMES`].
    pub fn from_dir(cwd: &Path) -> anyhow::Result<Option<Self>> {
        for file in find_project_files(cwd, SUPPORTED_FILE_NAMES) {
            if let Some(config) = Self::from_file(&file)? {
                return Ok(Some(config));
            }
        }

        Ok(None)
    }

    /// Loads a config from a file
    pub fn from_file(path: &Path) -> anyhow::Result<Option<Self>> {
        let s = std::fs::read_to_string(path)
            .with_context(|| format!("could not read config at `{}`", path.display()))?;

        match path.file_name() {
            Some(name) if name == CARGO_TOML => {
                let config = toml::from_str::<CargoTomlConfig>(&s)
                    .with_context(|| format!("could not parse config at `{}`", path.display()))?;
                let typos = config
                    .workspace
                    .and_then(|w| w.metadata.typope)
                    .or_else(|| config.package.and_then(|p| p.metadata.typope));

                if let Some(typos) = typos {
                    Ok(Some(typos))
                } else {
                    Ok(None)
                }
            }
            Some(name) if name == PYPROJECT_TOML => {
                let config = toml::from_str::<PyprojectTomlConfig>(&s)
                    .with_context(|| format!("could not parse config at `{}`", path.display()))?;

                if let Some(typos) = config.tool.typos {
                    Ok(Some(typos))
                } else {
                    Ok(None)
                }
            }
            _ => Self::from_toml(&s)
                .map(Some)
                .with_context(|| format!("could not parse config at `{}`", path.display())),
        }
    }

    /// Loads a config from TOML
    pub fn from_toml(data: &str) -> anyhow::Result<Self> {
        toml::from_str(data).map_err(Into::into)
    }

    /// Updates the config based on the value of another config
    pub fn update(&mut self, source: &Self) {
        self.files.update(&source.files);
        self.default.update(&source.default);
        self.type_.update(&source.type_);
    }

    /// Builds a [`WalkBuilder`] to find files based on the config
    pub fn to_walk_builder(&self, path: &Path) -> WalkBuilder {
        let mut walk = ignore::WalkBuilder::new(path);
        walk.skip_stdout(true)
            .git_global(self.files.ignore_global())
            .git_ignore(self.files.ignore_vcs())
            .git_exclude(self.files.ignore_vcs())
            .hidden(self.files.ignore_hidden())
            .parents(self.files.ignore_parent())
            .ignore(self.files.ignore_dot());

        walk
    }

    pub fn config_from_path(&self, path: impl AsRef<Path>) -> Cow<'_, EngineConfig> {
        let path = path.as_ref();
        let Some(extension) = path.extension() else {
            return Cow::Borrowed(&self.default);
        };
        let Some(lang) = Language::from_extension(extension) else {
            return Cow::Borrowed(&self.default);
        };

        let mut config = self.default.clone();
        if let Some(type_config) = self.type_.patterns.get(lang.name()) {
            config.update(type_config);
        }

        Cow::Owned(config)
    }
}

/// Defines how to ignore files from being checked by the linter
///
/// # Example
///
/// ```toml
/// [files]
/// ignore-hidden = false
/// ```
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[serde(rename_all = "kebab-case")]
pub struct Walk {
    /// Additional list of regexes to exclude files from being checked
    pub extend_exclude: Vec<String>,

    /// Skip hidden files and directories.
    pub ignore_hidden: Option<bool>,

    /// Respect ignore files.
    pub ignore_files: Option<bool>,

    /// Respect .ignore files.
    pub ignore_dot: Option<bool>,

    /// Respect ignore files in vcs directories.
    pub ignore_vcs: Option<bool>,

    /// Respect global ignore files.
    pub ignore_global: Option<bool>,

    /// Respect ignore files in parent directories.
    pub ignore_parent: Option<bool>,
}

impl Default for Walk {
    fn default() -> Self {
        Self {
            extend_exclude: Default::default(),
            ignore_hidden: Some(true),
            ignore_files: Some(true),
            ignore_dot: Some(true),
            ignore_vcs: Some(true),
            ignore_global: Some(true),
            ignore_parent: Some(true),
        }
    }
}

impl Walk {
    /// Updates the config based on the value of another config
    pub fn update(&mut self, source: &Self) {
        self.extend_exclude
            .extend(source.extend_exclude.iter().cloned());
        if let Some(source) = source.ignore_hidden {
            self.ignore_hidden = Some(source);
        }
        if let Some(source) = source.ignore_files {
            self.ignore_files = Some(source);
            self.ignore_dot = None;
            self.ignore_vcs = None;
            self.ignore_global = None;
            self.ignore_parent = None;
        }
        if let Some(source) = source.ignore_dot {
            self.ignore_dot = Some(source);
        }
        if let Some(source) = source.ignore_vcs {
            self.ignore_vcs = Some(source);
            self.ignore_global = None;
        }
        if let Some(source) = source.ignore_global {
            self.ignore_global = Some(source);
        }
        if let Some(source) = source.ignore_parent {
            self.ignore_parent = Some(source);
        }
    }

    /// Whether to skip hidden files and directories
    pub fn ignore_hidden(&self) -> bool {
        self.ignore_hidden.unwrap_or(true)
    }

    /// Whether to respect .ignore files
    pub fn ignore_dot(&self) -> bool {
        self.ignore_dot.or(self.ignore_files).unwrap_or(true)
    }

    /// Whether to respect ignore files in vcs directories
    pub fn ignore_vcs(&self) -> bool {
        self.ignore_vcs.or(self.ignore_files).unwrap_or(true)
    }

    /// Whether to respect global ignore files
    pub fn ignore_global(&self) -> bool {
        self.ignore_global
            .or(self.ignore_vcs)
            .or(self.ignore_files)
            .unwrap_or(true)
    }

    /// Whether to respect ignore files in parent directories
    pub fn ignore_parent(&self) -> bool {
        self.ignore_parent.or(self.ignore_files).unwrap_or(true)
    }
}

/// File type specific settings.
///
/// It helps a user define settings that only apply to some file types.
///
/// # Example
///
/// ```toml
/// [type.rust]
/// check-file = false
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[serde(transparent)]
pub struct TypeEngineConfig {
    /// Maps a file type to a custom config
    pub patterns: HashMap<String, EngineConfig>,
}

impl TypeEngineConfig {
    /// Updates the config based on the value of another config
    pub fn update(&mut self, source: &Self) {
        for (type_name, engine) in &source.patterns {
            self.patterns
                .entry(type_name.to_owned())
                .or_default()
                .update(engine);
        }
    }
}

/// Configuration for the linter's engine that can be applied globally or on a type of file
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[serde(rename_all = "kebab-case")]
pub struct EngineConfig {
    /// Whether to check files
    pub check_file: Option<bool>,

    /// Additional list of regexes to prevent strings from being checked
    #[serde(with = "serde_regex")]
    pub extend_ignore_re: Vec<regex::Regex>,
}

impl PartialEq for EngineConfig {
    fn eq(&self, other: &Self) -> bool {
        self.check_file == other.check_file
            && self
                .extend_ignore_re
                .iter()
                .map(|r| r.as_str())
                .eq(other.extend_ignore_re.iter().map(|r| r.as_str()))
    }
}

impl Eq for EngineConfig {}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            check_file: Some(true),
            extend_ignore_re: Default::default(),
        }
    }
}

impl EngineConfig {
    /// Updates the config based on the value of another config
    pub fn update(&mut self, source: &Self) {
        if let Some(source) = source.check_file {
            self.check_file = Some(source);
        }
    }

    /// Whether to check this file type
    pub fn check_file(&self) -> bool {
        self.check_file.unwrap_or(true)
    }
}

fn find_project_files<'a>(
    dir: &'a Path,
    names: &'a [&'a str],
) -> impl Iterator<Item = PathBuf> + 'a {
    names
        .iter()
        .map(|name| dir.join(name))
        .filter(|path| path.exists())
}

#[cfg(test)]
mod test {
    use std::path::Path;

    use tempfile::{tempdir, NamedTempFile};

    use super::{Config, EngineConfig};

    #[test]
    fn from_file() {
        let config = r#"
[files]
ignore-hidden = false

[default]
extend-ignore-re = ["some regex.*rrrregex"]

[type.cpp]
check-file = false
        "#;
        let file = NamedTempFile::new().unwrap();
        std::fs::write(file.path(), config).unwrap();
        let config = Config::from_file(file.path()).unwrap().unwrap();
        assert!(!config.files.ignore_hidden());
    }

    #[test]
    fn from_file_invalid() {
        let file = NamedTempFile::new().unwrap();
        std::fs::write(file.path(), "invaliddddddd").unwrap();
        Config::from_file(file.path()).unwrap_err();
        Config::from_file(Path::new("file that does not exist.toml")).unwrap_err();
    }

    #[test]
    fn from_dir() {
        let config = r#"
[files]
ignore-hidden = false

[default]
extend-ignore-re = ["some regex.*rrrregex"]

[type.cpp]
check-file = false
        "#;
        let dir: tempfile::TempDir = tempdir().unwrap();
        assert!(Config::from_dir(dir.path()).unwrap().is_none());

        let typos_config_file = dir.path().join(".typos.toml");
        std::fs::write(&typos_config_file, config).unwrap();
        let config = Config::from_dir(dir.path()).unwrap().unwrap();
        assert!(!config.files.ignore_hidden());
    }

    #[test]
    fn from_cargo_toml() {
        let config = r#"
[package]
name = "abc"
edition = "2021"
publish = false

[package.metadata.typope.files]
ignore-hidden = false

[package.metadata.typope.default]
extend-ignore-re = ["some regex.*rrrregex"]

[package.metadata.typope.type.cpp]
check-file = false
        "#;

        let dir: tempfile::TempDir = tempdir().unwrap();

        let cargo_toml = dir.path().join("Cargo.toml");
        std::fs::write(&cargo_toml, config).unwrap();
        let config = Config::from_file(&cargo_toml).unwrap().unwrap();
        assert!(!config.files.ignore_hidden());
    }

    #[test]
    fn test_update_from_nothing() {
        let defaulted = Config::default();

        let mut actual = defaulted.clone();
        actual.update(&Config::default());

        assert_eq!(actual, defaulted);
    }

    #[test]
    fn parse_extend_globs() {
        let input = r#"[type.po]
check-file = true
"#;
        let mut expected = Config::default();
        expected.type_.patterns.insert(
            "po".into(),
            EngineConfig {
                check_file: Some(true),
                ..Default::default()
            },
        );
        let actual = Config::from_toml(input).unwrap();
        assert_eq!(actual, expected);
    }
}
