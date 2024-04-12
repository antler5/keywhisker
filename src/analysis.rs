use anyhow::{Context, Result};
use keycat::{
    analysis::{Analyzer, MetricData as KcMetricData},
    Corpus, Layout, NgramType,
};
use keymeow::{LayoutData, MetricContext, MetricData};
use linya::{Bar, Progress};
use rand::seq::SliceRandom;
use rand::thread_rng;
use rayon::prelude::*;
use std::{fs::File, io::Write};
use std::{fs::OpenOptions, io::LineWriter, sync::Mutex};

pub struct LayoutTotals {
    chars: u32,
    bigrams: u32,
    skipgrams: u32,
    trigrams: u32,
}

impl LayoutTotals {
    pub fn new(l: &Layout, c: &Corpus) -> Self {
        LayoutTotals {
            chars: l.total_char_count(&c),
            bigrams: l.total_bigram_count(&c),
            skipgrams: l.total_skipgram_count(&c),
            trigrams: l.total_trigram_count(&c)
        }
    }
    pub fn percentage(&self, freq: f32, kind: NgramType) -> f32 {
        let denom = match kind {
            NgramType::Monogram => self.chars,
            NgramType::Bigram => self.bigrams,
            NgramType::Skipgram => self.skipgrams,
            NgramType::Trigram => self.trigrams,
        } as f32;
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
    count: u64,
    char_set: &str,
) -> Result<()> {
    let metrics: Result<Vec<usize>, _> = metric_names
        .iter()
        .map(|s| get_metric(s, &metric_data))
        .collect();
    let metrics = metrics.context("invalid metric")?;
    let layout = Layout {
        matrix: char_set.chars().map(|c| corpus.corpus_char(c)).collect()
    };

    let totals = LayoutTotals::new(&layout, &corpus);

    let data = filter_metrics(kc_metric_data(metric_data, layout.matrix.len()), &metrics);
    let analyzer = Analyzer::from(data, corpus);

    let file = File::create("data/data.csv").context("couldn't create data file")?;
    let mut writer = LineWriter::new(file);

    for m in &metric_names {
        write!(writer, "{m},")?;
    }
    writeln!(writer)?;
    let progress = Mutex::new(Progress::new());
    let bar: Bar = progress.lock().unwrap().bar(count.try_into()?, "Analyzing");

    let threads: u64 = 64;
    (0..threads)
        .into_par_iter()
        .try_for_each(|_| -> Result<()> {
            let mut stats = analyzer.calc_stats(&layout);
            let mut layout = layout.clone();
            let mut rng = thread_rng();
            let file = OpenOptions::new()
                .create(false)
                .append(true)
                .open("data/data.csv")?;
            let mut writer = LineWriter::new(file);
            for _ in 0..count / threads {
                layout.matrix.shuffle(&mut rng);
                stats.iter_mut().for_each(|x| *x = 0.0);
                analyzer.recalc_stats(&mut stats, &layout);
                let mut s = String::new();
                for m in &metrics {
                    let percent = totals.percentage(stats[*m].into(), analyzer.data.metrics[*m]);
                    s.push_str(&percent.to_string());
                    s.push(',');
                }
                s.push('\n');
                writer.write_all(&s.into_bytes())?;
                progress.lock().unwrap().inc_and_draw(&bar, 1);
            }
            Ok(())
        })?;

    Ok(())
}

pub fn stats(metric_data: MetricData, corpus: Corpus, layout: LayoutData) -> Result<()> {
    let ctx = MetricContext::new(&layout, metric_data, corpus)
        .context("could not produce metric context")?;
    let totals = LayoutTotals::new(&ctx.layout, &ctx.analyzer.corpus);

    let stats = ctx.analyzer.calc_stats(&ctx.layout);
    for (i, stat) in stats.iter().enumerate() {
        println!(
            "{}: {:.2}%",
            ctx.metrics[i].name,
            totals.percentage(*stat, ctx.metrics[i].ngram_type)
        );
    }
    Ok(())
}
