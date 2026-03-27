use std::path::PathBuf;

use clap::Parser;

use crate::{config, error::Result};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long)]
    generate_config: bool,
    #[arg(long)]
    print_config_path: bool,
    #[arg(long)]
    print_default_config: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StartupAction {
    Run,
    Exit,
}

pub(crate) fn initialize() -> Result<StartupAction> {
    let cli = Cli::parse();

    if cli.generate_config {
        let target = config::resolved_write_path(cli.config.as_deref())?;
        config::write_default_config(&target)?;
        println!("wrote default config to {}", target.display());
        return Ok(StartupAction::Exit);
    }

    if cli.print_config_path {
        let path = config::resolved_write_path(cli.config.as_deref())?;
        println!("{}", path.display());
        return Ok(StartupAction::Exit);
    }

    if cli.print_default_config {
        print!("{}", config::render_default_config()?);
        return Ok(StartupAction::Exit);
    }

    let loaded = config::load(cli.config.as_deref())?;
    config::install_runtime(loaded);
    Ok(StartupAction::Run)
}
