mod analysis;

use analysis::output_table;
use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};
use keycat::Corpus;
use keymeow::LayoutData;
use km_data::Data as KeymeowData;

pub fn print_matrix(letters: &[char]) {
    for row in 0..3 {
        for col in 0..5 {
            print!("{} ", letters[col * 3 + row]);
        }
        print!(" ");
        for col in 5..10 {
            print!("{} ", letters[col * 3 + row]);
        }
        println!();
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

impl AnalysisArgs {
    pub fn get(&self, data: &KeymeowData) -> Result<(keycat::Corpus, keymeow::MetricData)> {
        Ok((
            data.get_corpus(&self.corpus)?,
            data.get_metrics(&self.keyboard)?,
        ))
    }
}

#[derive(ValueEnum, Debug, Clone)]
enum GenerationStrategy {
    GreedyDeterministic,
    GreedyNaive,
    SimulatedAnnealing,
}

#[derive(Subcommand)]
enum Commands {
    /// Display information about the environment (e.g. available layouts, corpora)
    Env,
    /// Collect metric data into a csv
    Collect {
        /// The total number of layouts to analyze
        count: u64,
        /// The set of characters to use as keys in the randomized layouts
        char_set: String,
        /// The list of metrics to collect data for
        metrics: Vec<String>,
        #[command(flatten)]
        analysis_args: AnalysisArgs,
    },
    Stats {
        layout: String,
        #[command(flatten)]
        analysis_args: AnalysisArgs,
    },
    Corpus {
        name: String,
    },
    RunGeneration {
        /// The number of generation runs to perform
        runs: u64,
        /// The generation strategy to use
        #[clap(value_enum)]
        strategy: GenerationStrategy,
        /// The set of characters to use as keys in the layout
        char_set: String,
        /// The metric to reduce
        metric: String,
        #[command(flatten)]
        analysis_args: AnalysisArgs,
    },
    FormatLayout {
        chars: String,
    },
    LayoutData {
        chars: String,
        keyboard: String,
        #[arg(short, long)]
        name: Option<String>,
    },
}

fn main() -> Result<()> {
    let keymeow = KeymeowData::with_download()?;
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Env) => {
            println!(
                "Corpora: {:?}",
                keymeow.corpora.keys().collect::<Vec<_>>()
            );
            println!(
                "Keyboards: {:?}",
                keymeow.keyboards.keys().collect::<Vec<_>>()
            );
            println!(
                "Layouts: {:?}",
                keymeow.layouts.keys().collect::<Vec<_>>()
            );
        }
        Some(Commands::Collect {
            count,
            char_set,
            metrics,
            analysis_args,
        }) => {
            let (corpus, metric_data) = analysis_args.get(&keymeow)?;
            output_table(metrics.to_owned(), metric_data, corpus, *count, char_set)?
        }
        Some(Commands::Stats {
            layout,
            analysis_args,
        }) => {
            let (corpus, metric_data) = analysis_args.get(&keymeow)?;
            let layout = keymeow.get_layout(layout)?;
            analysis::stats(metric_data, corpus, layout)?;
        }
        Some(Commands::Corpus { name }) => {
            let corpus = keymeow.get_corpus(name)?;
            println!("{:?}", corpus.trigrams);
            println!("Size: {:?} bytes", std::mem::size_of_val(&*corpus.trigrams));
            println!("Length: {:?}", corpus.trigrams.len());
        }
        Some(Commands::RunGeneration {
            runs,
            strategy,
            char_set,
            metric,
            analysis_args,
        }) => {
            let (corpus, metric_data) = analysis_args.get(&keymeow)?;
            crate::analysis::output_generation(
                metric,
                metric_data,
                corpus,
                char_set,
                strategy,
                *runs,
            )?;
        }
        Some(Commands::FormatLayout { chars }) => {
            print_matrix(chars.chars().collect::<Vec<_>>().as_ref());
        }
        Some(Commands::LayoutData {
            chars,
            keyboard,
            name,
        }) => {
            let corpus = Corpus::with_char_list(chars.chars().map(|c| vec![c]).collect());
            let metrics = keymeow.get_metrics(keyboard)?;
            let layout = keycat::Layout {
                matrix: chars
                    .chars()
                    .map(|c| corpus.corpus_char(c))
                    .collect(),
            };
            let data = LayoutData::from_keyboard_layout(&metrics.keyboard, &layout, &corpus).name(
                match name {
                    Some(name) => name.to_owned(),
                    None => "Custom".to_string(),
                },
            );
            println!("{}", serde_json::to_string_pretty(&data)?);
        }
        None => {}
    };
    Ok(())
}
