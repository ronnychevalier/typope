use std::fs::Metadata;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Context;

use ignore::DirEntry;

use rayon::iter::{ParallelBridge, ParallelIterator};

use typope::config;
use typope::config::Config;
use typope::lang::Language;
use typope::lint::{Linter, TypoFixer};

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
#[command(group = clap::ArgGroup::new("mode").multiple(false))]
pub(crate) struct Args {
    /// Paths to check
    #[arg(default_value = ".")]
    path: Vec<PathBuf>,

    /// Debug: Print each file that would be spellchecked.
    #[arg(long, group = "mode", help_heading = "Mode")]
    files: bool,

    /// Debug: Print each string that would be spellchecked.
    #[arg(long, group = "mode", help_heading = "Mode")]
    strings: bool,

    /// Write fixes out
    #[arg(long, short, group = "mode", help_heading = "Mode")]
    write_changes: bool,

    /// Write the current configuration to file with `-` for stdout
    #[arg(long, value_name = "OUTPUT", group = "mode", help_heading = "Mode")]
    dump_config: Option<PathBuf>,

    /// Show all supported file types
    #[arg(long, group = "mode", help_heading = "Mode")]
    type_list: bool,

    /// Sort results
    #[arg(long, help_heading = "Output")]
    sort: bool,

    /// Render style for messages
    #[arg(
        long,
        value_enum,
        ignore_case = true,
        default_value("long"),
        help_heading = "Output"
    )]
    format: Format,

    #[command(flatten, next_help_heading = "Config")]
    walk: WalkArgs,
}

impl Args {
    #[allow(clippy::print_stderr, clippy::print_stdout)]
    pub fn run(self) -> anyhow::Result<()> {
        if let Some(output_path) = &self.dump_config {
            return self.run_dump_config(output_path);
        }
        if self.type_list {
            for lang in Language::iter() {
                println!("{}: {}", lang.name(), lang.detections().join(", "));
            }
            return Ok(());
        }

        let report_handler = self.format().into_error_hook();
        miette::set_hook(report_handler)?;

        let config = self.to_config()?;
        let walker = self.to_walk(&config)?;
        let process_entry = |file: DirEntry| {
            let config = config.config_from_path(file.path());
            if !config.check_file() {
                return 0;
            }

            let Ok(Some(mut linter)) = Linter::from_path(file.path()) else {
                return 0;
            };
            if self.strings {
                let mut stdout = std::io::stdout().lock();
                for string in linter.strings() {
                    let _ = writeln!(stdout, "{string}");
                }
                return 0;
            }
            if self.files {
                println!("{}", file.path().display());
                return 0;
            }
            linter.extend_ignore_re(&config.extend_ignore_re);

            let mut stderr = std::io::stderr().lock();

            let mut fixer = None;

            linter
                .iter()
                .map(|typo| {
                    if self.write_changes {
                        if let Ok(fixer) = fixer.get_or_insert_with(|| TypoFixer::new(file.path()))
                        {
                            let _ = fixer.fix(typo.as_ref());
                        }
                    }

                    let typo: miette::Report = typo.into();
                    let _ = writeln!(stderr, "{typo:?}");
                })
                .count()
        };
        let typos_found: usize = if self.sort() {
            walker.map(process_entry).sum()
        } else {
            walker.par_bridge().map(process_entry).sum()
        };

        if typos_found > 0 {
            std::process::exit(1);
        } else {
            Ok(())
        }
    }

    fn run_dump_config(&self, output_path: &Path) -> anyhow::Result<()> {
        let config = self.to_config()?;
        let output = toml::to_string_pretty(&config)?;
        if output_path == Path::new("-") {
            std::io::stdout().write_all(output.as_bytes())?;
        } else {
            std::fs::write(output_path, &output)?;
        }

        Ok(())
    }

    pub fn to_walk<'a>(
        &'a self,
        config: &'a Config,
    ) -> anyhow::Result<impl Iterator<Item = DirEntry> + 'a> {
        let mut overrides = ignore::overrides::OverrideBuilder::new(".");
        for pattern in &config.files.extend_exclude {
            overrides.add(&format!("!{pattern}"))?;
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

    pub fn to_config(&self) -> anyhow::Result<config::Config> {
        let config_from_args = config::Config {
            files: self.walk.to_config(),
            ..Default::default()
        };

        let cwd = std::env::current_dir().context("no current working directory")?;
        let mut config = Config::default();
        for ancestor in cwd.ancestors() {
            if let Some(derived) = Config::from_dir(ancestor)? {
                config.update(&derived);
                break;
            }
        }
        config.update(&config_from_args);
        Ok(config)
    }

    /// Whether to sort results
    pub fn sort(&self) -> bool {
        self.sort
    }
}

#[derive(clap::Args)]
struct WalkArgs {
    /// Ignore files and directories matching the glob.
    #[arg(long, value_name = "GLOB")]
    exclude: Vec<String>,

    /// Search hidden files and directories
    #[arg(long, short = 'H')]
    hidden: bool,
    #[arg(long, overrides_with("hidden"), hide = true)]
    no_hidden: bool,

    /// Don't respect ignore files
    #[arg(long, short = 'I')]
    no_ignore: bool,
    #[arg(long, overrides_with("no_ignore"), hide = true)]
    ignore: bool,

    /// Don't respect .ignore files
    #[arg(long)]
    no_ignore_dot: bool,
    #[arg(long, overrides_with("no_ignore_dot"), hide = true)]
    ignore_dot: bool,

    /// Don't respect global ignore files
    #[arg(long)]
    no_ignore_global: bool,
    #[arg(long, overrides_with("no_ignore_global"), hide = true)]
    ignore_global: bool,

    /// Don't respect ignore files in parent directories
    #[arg(long)]
    no_ignore_parent: bool,
    #[arg(long, overrides_with("no_ignore_parent"), hide = true)]
    ignore_parent: bool,

    /// Don't respect ignore files in vcs directories
    #[arg(long)]
    no_ignore_vcs: bool,
    #[arg(long, overrides_with("no_ignore_vcs"), hide = true)]
    ignore_vcs: bool,
}

impl WalkArgs {
    pub fn to_config(&self) -> config::Walk {
        config::Walk {
            extend_exclude: self.exclude.clone(),
            ignore_hidden: self.ignore_hidden(),
            ignore_files: self.ignore_files(),
            ignore_dot: self.ignore_dot(),
            ignore_vcs: self.ignore_vcs(),
            ignore_global: self.ignore_global(),
            ignore_parent: self.ignore_parent(),
        }
    }

    fn ignore_hidden(&self) -> Option<bool> {
        resolve_bool_arg(self.no_hidden, self.hidden)
    }

    fn ignore_files(&self) -> Option<bool> {
        resolve_bool_arg(self.ignore, self.no_ignore)
    }

    fn ignore_dot(&self) -> Option<bool> {
        resolve_bool_arg(self.ignore_dot, self.no_ignore_dot)
    }

    fn ignore_vcs(&self) -> Option<bool> {
        resolve_bool_arg(self.ignore_vcs, self.no_ignore_vcs)
    }

    fn ignore_global(&self) -> Option<bool> {
        resolve_bool_arg(self.ignore_global, self.no_ignore_global)
    }

    fn ignore_parent(&self) -> Option<bool> {
        resolve_bool_arg(self.ignore_parent, self.no_ignore_parent)
    }
}

fn resolve_bool_arg(yes: bool, no: bool) -> Option<bool> {
    match (yes, no) {
        (true, false) => Some(true),
        (false, true) => Some(false),
        (_, _) => None,
    }
}
