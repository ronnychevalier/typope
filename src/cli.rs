use std::fs::Metadata;
use std::path::Path;
use std::path::PathBuf;

use ignore::{DirEntry, WalkBuilder};

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

#[derive(clap::Args)]
struct WalkArgs {
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

impl Args {
    pub fn to_walk_builder(&self, path: &Path) -> WalkBuilder {
        let mut walk = ignore::WalkBuilder::new(path);
        walk.skip_stdout(true)
            .git_global(
                !(self.walk.no_ignore_global || self.walk.no_ignore_vcs || self.walk.no_ignore),
            )
            .git_ignore(!self.walk.no_ignore_vcs || self.walk.no_ignore)
            .git_exclude(!self.walk.no_ignore_vcs || self.walk.no_ignore)
            .hidden(self.walk.hidden)
            .parents(!(self.walk.no_ignore_parent || self.walk.no_ignore))
            .ignore(!(self.walk.no_ignore_dot || self.walk.no_ignore));
        if self.sort {
            walk.sort_by_file_name(|a, b| a.cmp(b));
        }

        walk
    }

    pub fn to_walk(&self) -> impl Iterator<Item = DirEntry> + '_ {
        self.path.iter().flat_map(|path| {
            self.to_walk_builder(path)
                .build()
                .filter_map(Result::ok)
                .filter(|entry| {
                    entry
                        .metadata()
                        .as_ref()
                        .map(Metadata::is_file)
                        .unwrap_or(false)
                })
        })
    }

    pub fn format(&self) -> Format {
        self.format
    }
}
