mod analysis;
mod files;

use analysis::output_table;
use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use directories::BaseDirs;
use std::{collections::HashMap, fs, path::PathBuf};

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
    fn get_corpus(&self, s: &str) -> Result<keycat::Corpus> {
        let path = self.corpora.get(s).context("couldn't find corpus")?;
        let b = fs::read(path).with_context(|| format!("couldn't read corpus file {path:?}"))?;
        rmp_serde::from_slice(&b).context("couldn't deserialize corpus")
    }
    fn get_metrics(&self, s: &str) -> Result<keymeow::MetricData> {
        let path = self.keyboards.get(s).context("couldn't find keyboard")?;
        let b = fs::read(path).with_context(|| format!("couldn't read metrics file {path:?}"))?;
        rmp_serde::from_slice(&b).context("couldn't deserialize metrics")
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Args)]
pub struct AnalysisArgs {
    /// The corpus to use for analysis
    #[arg(short, long)]
    corpus: String,
    /// The keyboard to use for analysis
    #[arg(short, long)]
    keyboard: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Display information about the environment (e.g. available layouts, corpora)
    Env,
    /// Collect metric data into a csv
    Collect {
        /// The total number of layouts to analyze
        count: usize,
        /// The list of metrics to collect data for
        metrics: Vec<String>,
        #[command(flatten)]
        analysis_args: AnalysisArgs,
    },
}

fn main() -> Result<()> {
    let keywhisker = Keywhisker::new()?;
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Env) => {
            println!(
                "Corpora: {:?}",
                keywhisker.corpora.keys().collect::<Vec<_>>()
            );
            println!(
                "Keyboards: {:?}",
                keywhisker.keyboards.keys().collect::<Vec<_>>()
            );
        }
        Some(Commands::Collect {
            count,
            metrics,
            analysis_args,
        }) => output_table(
            metrics.to_owned(),
            keywhisker.get_metrics(&analysis_args.keyboard)?,
            keywhisker.get_corpus(&analysis_args.corpus)?,
            *count,
        )?,
        None => {}
    };
    Ok(())
}
