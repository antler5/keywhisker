mod files;
mod analysis;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use directories::BaseDirs;
use std::{collections::HashMap, path::PathBuf};

pub struct Keywhisker {
    pub corpora: HashMap<String, PathBuf>,
    pub keyboards: HashMap<String, PathBuf>,
}

impl Keywhisker {
    fn new() -> Result<Self> {
        let base_dirs = BaseDirs::new().context("couldn't determine base directories")?;
        let data_dir = base_dirs.data_dir().join("keymeow");
        Ok(Keywhisker {
            corpora: files::dir_to_hashmap(&data_dir.join("corpora"))?,
            keyboards: files::dir_to_hashmap(&data_dir.join("metrics"))?,
        })
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>
}

#[derive(Subcommand)]
enum Commands {
    /// Display information about the environment (e.g. available layouts, corpora)
    Env,
    /// Calculate average metrics in entire layout search space
    Average,
}

fn main() -> Result<()> {
    let keywhisker = Keywhisker::new()?;
    let cli = Cli::parse();

    match &cli.command {
	Some(Commands::Env) => {
	    println!("Corpora: {:?}", keywhisker.corpora.keys().collect::<Vec<_>>());
	    println!("Keyboards: {:?}", keywhisker.keyboards.keys().collect::<Vec<_>>());
	},
	Some(Commands::Average) => {
	    
	},
	None => {}
    };
    Ok(())
}
