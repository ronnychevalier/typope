//! Config parsers to recognize the config fields of [`typos`](https://crates.io/crates/typos-cli).
// It is based on <https://github.com/crate-ci/typos/blob/master/crates/typos-cli/src/config.rs>
// but it has been modified to remove fields that we do not care about for the moment.
use std::borrow::Cow;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use anyhow::Context;

use ignore::WalkBuilder;

use crate::lang::Language;

/// List of file names that can contain the configuration
pub const SUPPORTED_FILE_NAMES: &[&str] =
    &["typos.toml", "_typos.toml", ".typos.toml", "pyproject.toml"];

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

        if path.file_name() == Some(OsStr::new("pyproject.toml")) {
            let config = toml::from_str::<PyprojectTomlConfig>(&s)
                .with_context(|| format!("could not parse config at `{}`", path.display()))?;

            if config.tool.typos.is_none() {
                Ok(None)
            } else {
                Ok(config.tool.typos)
            }
        } else {
            Self::from_toml(&s)
                .map(Some)
                .with_context(|| format!("could not parse config at `{}`", path.display()))
        }
    }

    /// Loads a config from TOML
    pub fn from_toml(data: &str) -> anyhow::Result<Self> {
        toml::from_str(data).map_err(Into::into)
    }

    pub fn from_defaults() -> Self {
        Self {
            files: Walk::from_defaults(),
            default: EngineConfig::from_defaults(),
            type_: TypeEngineConfig::from_defaults(),
        }
    }

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
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

impl Walk {
    pub fn from_defaults() -> Self {
        let empty = Self::default();
        Self {
            extend_exclude: empty.extend_exclude.clone(),
            ignore_hidden: Some(empty.ignore_hidden()),
            ignore_files: Some(true),
            ignore_dot: Some(empty.ignore_dot()),
            ignore_vcs: Some(empty.ignore_vcs()),
            ignore_global: Some(empty.ignore_global()),
            ignore_parent: Some(empty.ignore_parent()),
        }
    }

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
    pub patterns: HashMap<String, EngineConfig>,
}

impl TypeEngineConfig {
    pub fn from_defaults() -> Self {
        Self::default()
    }

    pub fn update(&mut self, source: &Self) {
        for (type_name, engine) in &source.patterns {
            self.patterns
                .entry(type_name.to_owned())
                .or_default()
                .update(engine);
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[serde(rename_all = "kebab-case")]
pub struct EngineConfig {
    /// Verifying spelling in files.
    pub check_file: Option<bool>,

    /// Additional list of regexes to prevent string from being checked
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

impl EngineConfig {
    pub fn from_defaults() -> Self {
        let empty = Self::default();
        Self {
            check_file: Some(empty.check_file()),
            ..Default::default()
        }
    }

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
        let dir = tempdir().unwrap();
        assert!(Config::from_dir(dir.path()).unwrap().is_none());

        let typos_config_file = dir.path().join(".typos.toml");
        std::fs::write(&typos_config_file, config).unwrap();
        let config = Config::from_dir(dir.path()).unwrap().unwrap();
        assert!(!config.files.ignore_hidden())
    }

    #[test]
    fn test_update_from_nothing() {
        let null = Config::default();
        let defaulted = Config::from_defaults();

        let mut actual = defaulted.clone();
        actual.update(&null);

        assert_eq!(actual, defaulted);
    }

    #[test]
    fn test_update_from_defaults() {
        let null = Config::default();
        let defaulted = Config::from_defaults();

        let mut actual = null;
        actual.update(&defaulted);

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
