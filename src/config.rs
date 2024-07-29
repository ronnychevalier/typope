//! Config parsers to recognize the config fields of [`typos`](https://crates.io/crates/typos-cli).
// It is based on <https://github.com/crate-ci/typos/blob/master/crates/typos-cli/src/config.rs>
// but it has been modified to remove fields that we do not care about for the moment.
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use anyhow::Context;

use ignore::WalkBuilder;

use crate::lang::Language;

const NO_CHECK_TYPES: &[&str] = &["cert", "lock"];

pub const SUPPORTED_FILE_NAMES: &[&str] =
    &["typos.toml", "_typos.toml", ".typos.toml", "pyproject.toml"];

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    pub files: Walk,
    pub default: EngineConfig,
    #[serde(rename = "type")]
    pub type_: TypeEngineConfig,
    #[serde(skip)]
    pub overrides: EngineConfig,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[serde(rename_all = "kebab-case")]
pub struct PyprojectTomlConfig {
    pub tool: PyprojectTomlTool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[serde(rename_all = "kebab-case")]
pub struct PyprojectTomlTool {
    pub typos: Option<Config>,
}

impl Config {
    pub fn from_dir(cwd: &Path) -> anyhow::Result<Option<Self>> {
        for file in find_project_files(cwd, SUPPORTED_FILE_NAMES) {
            if let Some(config) = Self::from_file(&file)? {
                return Ok(Some(config));
            }
        }

        Ok(None)
    }

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

    pub fn from_toml(data: &str) -> anyhow::Result<Self> {
        toml::from_str(data).map_err(Into::into)
    }

    pub fn from_defaults() -> Self {
        Self {
            files: Walk::from_defaults(),
            default: EngineConfig::from_defaults(),
            type_: TypeEngineConfig::from_defaults(),
            overrides: EngineConfig::default(),
        }
    }

    pub fn update(&mut self, source: &Self) {
        self.files.update(&source.files);
        self.default.update(&source.default);
        self.type_.update(&source.type_);
        self.overrides.update(&source.overrides);
    }

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
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(default)]
#[serde(rename_all = "kebab-case")]
pub struct Walk {
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

    pub fn extend_exclude(&self) -> &[String] {
        &self.extend_exclude
    }

    pub fn ignore_hidden(&self) -> bool {
        self.ignore_hidden.unwrap_or(true)
    }

    pub fn ignore_dot(&self) -> bool {
        self.ignore_dot.or(self.ignore_files).unwrap_or(true)
    }

    pub fn ignore_vcs(&self) -> bool {
        self.ignore_vcs.or(self.ignore_files).unwrap_or(true)
    }

    pub fn ignore_global(&self) -> bool {
        self.ignore_global
            .or(self.ignore_vcs)
            .or(self.ignore_files)
            .unwrap_or(true)
    }

    pub fn ignore_parent(&self) -> bool {
        self.ignore_parent.or(self.ignore_files).unwrap_or(true)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[serde(transparent)]
pub struct TypeEngineConfig {
    pub patterns: HashMap<String, EngineConfig>,
}

impl TypeEngineConfig {
    pub fn from_defaults() -> Self {
        let mut patterns = HashMap::new();

        for &no_check_type in NO_CHECK_TYPES {
            patterns.insert(
                no_check_type.to_owned(),
                EngineConfig {
                    check_file: Some(false),
                },
            );
        }

        Self { patterns }
    }

    pub fn update(&mut self, source: &Self) {
        for (type_name, engine) in &source.patterns {
            self.patterns
                .entry(type_name.to_owned())
                .or_default()
                .update(engine);
        }
    }

    pub fn config_from_path(&self, path: impl AsRef<Path>) -> Option<&EngineConfig> {
        let path = path.as_ref();
        let extension = path.extension()?;
        let lang = Language::from_extension(extension)?;
        self.patterns.get(lang.name())
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(default)]
#[serde(rename_all = "kebab-case")]
pub struct EngineConfig {
    /// Verifying spelling in files.
    pub check_file: Option<bool>,
}

impl EngineConfig {
    pub fn from_defaults() -> Self {
        let empty = Self::default();
        Self {
            check_file: Some(empty.check_file()),
        }
    }

    pub fn update(&mut self, source: &Self) {
        if let Some(source) = source.check_file {
            self.check_file = Some(source);
        }
    }

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
    use super::*;

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
            },
        );
        let actual = Config::from_toml(input).unwrap();
        assert_eq!(actual, expected);
    }
}
