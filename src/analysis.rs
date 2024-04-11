use anyhow::{Context, Result};
use keycat::{
    analysis::{Analyzer, MetricData as KcMetricData},
    Corpus, Layout, NgramType,
};
use keymeow::{LayoutData, MetricContext, MetricData};
use linya::{Bar, Progress};
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::io::LineWriter;
use std::{fs::File, io::Write};

fn total_ngram_count(list: &[u32]) -> u64 {
    list.iter().map(|x| *x as u64).sum()
}

pub struct NgramTotals {
    chars: u64,
    bigrams: u64,
    skipgrams: u64,
    trigrams: u64,
}

impl NgramTotals {
    pub fn new(c: &Corpus) -> Self {
        NgramTotals {
            chars: total_ngram_count(&c.chars),
            bigrams: total_ngram_count(&c.bigrams),
            skipgrams: total_ngram_count(&c.skipgrams),
            trigrams: total_ngram_count(&c.trigrams),
        }
    }
    pub fn percentage(&self, freq: f64, kind: NgramType) -> f64 {
        let denom = match kind {
            NgramType::Monogram => self.chars,
            NgramType::Bigram => self.bigrams,
            NgramType::Skipgram => self.skipgrams,
            NgramType::Trigram => self.trigrams,
        } as f64;
        100. * freq / denom
    }
}

pub fn kc_metric_data(metric_data: keymeow::MetricData, position_count: usize) -> KcMetricData {
    KcMetricData::from(
        metric_data.metrics.iter().map(|m| m.ngram_type).collect(),
        metric_data.strokes,
        position_count,
    )
}

pub fn get_metric(s: &str, data: &MetricData) -> Result<usize> {
    data.metrics
        .iter()
        .enumerate()
        .find(|(_, m)| m.name == s || m.short == s)
        .map(|(i, _)| i)
        .context("metric not found")
}

pub fn filter_metrics(md: KcMetricData, metrics: &[usize]) -> KcMetricData {
    KcMetricData {
        strokes: md
            .strokes
            .into_iter()
            .filter(|ns| ns.amounts.iter().any(|amt| metrics.contains(&amt.metric)))
            .collect(),
        ..md
    }
}

pub fn output_table(
    metric_names: Vec<String>,
    metric_data: keymeow::MetricData,
    corpus: Corpus,
    count: usize,
) -> Result<()> {
    let metrics: Result<Vec<usize>, _> = metric_names
        .iter()
        .map(|s| get_metric(s, &metric_data))
        .collect();
    let metrics = metrics.context("invalid metric")?;
    let mut layout = Layout {
        matrix: (0..corpus.char_list.len()).collect(),
    };

    let totals = NgramTotals::new(&corpus);

    let data = filter_metrics(kc_metric_data(metric_data, layout.matrix.len()), &metrics);
    let analyzer = Analyzer::from(data, corpus);

    let file = File::create("data/data.csv").context("couldn't create data file")?;
    let mut file = LineWriter::new(file);

    for m in &metric_names {
        write!(file, "{m},")?;
    }
    write!(file, "\n")?;
    let mut rng = thread_rng();
    let mut progress = Progress::new();
    let bar: Bar = progress.bar(count, "Analyzing");

    for _ in 0..count {
        layout.matrix.shuffle(&mut rng);
        let stats = analyzer.calc_stats(&layout);
        let mut s = String::new();
        for m in &metrics {
            let percent = totals.percentage(stats[*m].into(), analyzer.data.metrics[*m]);
            s.push_str(&percent.to_string());
            s.push(',');
        }
        s.push('\n');
        file.write_all(&s.into_bytes())?;
        progress.inc_and_draw(&bar, 1);
    }
    Ok(())
}

pub fn stats(metric_data: MetricData, corpus: Corpus, layout: LayoutData) -> Result<()> {
    let totals = NgramTotals::new(&corpus);

    let ctx = MetricContext::new(&layout, metric_data, corpus)
        .context("could not produce metric context")?;
    let stats = ctx.analyzer.calc_stats(&ctx.layout);
    for (i, stat) in stats.iter().enumerate() {
        println!(
            "{}: {:.2}%",
            ctx.metrics[i].name,
            totals.percentage(*stat as f64, ctx.analyzer.data.metrics[i])
        );
    }
    Ok(())
}
