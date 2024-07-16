use std::fs::Metadata;
use std::path::PathBuf;

use ignore::DirEntry;

use orthotypos::config;
use orthotypos::config::Config;

#[derive(Copy, Clone, PartialEq, Eq, clap::ValueEnum, Default)]
pub enum Format {
    #[default]
    Long,
    Json,
}

impl Format {
    pub fn into_error_hook(self) -> miette::ErrorHook {
        match self {
            Self::Long => Box::new(|_| Box::new(miette::GraphicalReportHandler::new())),
            Self::Json => Box::new(|_| Box::new(miette::JSONReportHandler::new())),
        }
    }
}

#[derive(clap::Parser)]
#[command(about, version)]
pub(crate) struct Args {
    /// Paths to check
    #[arg(default_value = ".")]
    path: Vec<PathBuf>,

    /// Sort results
    #[arg(long)]
    sort: bool,

    /// Render style for messages
    #[arg(long, value_enum, ignore_case = true, default_value("long"))]
    format: Format,

    #[command(flatten)]
    walk: WalkArgs,
}

impl Args {
    pub fn to_walk<'a>(
        &'a self,
        config: &'a Config,
    ) -> anyhow::Result<impl Iterator<Item = DirEntry> + 'a> {
        let mut overrides = ignore::overrides::OverrideBuilder::new(".");
        for pattern in &config.files.extend_exclude {
            overrides.add(&format!("!{}", pattern))?;
        }
        let overrides = overrides.build()?;

        Ok(self.path.iter().flat_map(move |path| {
            let mut walk = config.to_walk_builder(path);
            if self.sort {
                walk.sort_by_file_name(|a, b| a.cmp(b));
            }
            if !config.files.extend_exclude.is_empty() {
                walk.overrides(overrides.clone());
            }
            walk.build().filter_map(Result::ok).filter(|entry| {
                entry
                    .metadata()
                    .as_ref()
                    .map(Metadata::is_file)
                    .unwrap_or(false)
            })
        }))
    }

    pub fn format(&self) -> Format {
        self.format
    }

    pub fn to_config(&self) -> config::Config {
        config::Config {
            files: self.walk.to_config(),
            ..Default::default()
        }
    }
}

#[derive(clap::Args)]
struct WalkArgs {
    /// Ignore files and directories matching the glob.
    #[arg(long, value_name = "GLOB")]
    exclude: Vec<String>,

    /// Search hidden files and directories
    #[arg(long)]
    hidden: bool,

    /// Don't respect ignore files
    #[arg(long)]
    no_ignore: bool,

    /// Don't respect .ignore files
    #[arg(long)]
    no_ignore_dot: bool,

    /// Don't respect global ignore files
    #[arg(long)]
    no_ignore_global: bool,

    /// Don't respect ignore files in parent directories
    #[arg(long)]
    no_ignore_parent: bool,

    /// Don't respect ignore files in vcs directories
    #[arg(long)]
    no_ignore_vcs: bool,
}

impl WalkArgs {
    pub fn to_config(&self) -> config::Walk {
        config::Walk {
            extend_exclude: self.exclude.clone(),
            ignore_hidden: Some(self.hidden),
            ignore_files: Some(!self.no_ignore),
            ignore_dot: Some(!self.no_ignore_dot),
            ignore_vcs: Some(!self.no_ignore_vcs),
            ignore_global: Some(!self.no_ignore_global),
            ignore_parent: Some(!self.no_ignore_parent),
        }
    }
}
