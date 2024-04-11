use anyhow::{Context, Result};
use keycat::{
    analysis::{Analyzer, MetricData as KcMetricData},
    Corpus, Layout,
};
use keymeow::MetricData;
use rand::seq::SliceRandom;
use rand::thread_rng;
use linya::{Progress, Bar};
use std::io::LineWriter;
use std::{fs::File, io::Write};

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
            s.push_str(&stats[*m].to_string());
            s.push(',');
        }
        s.push('\n');
        file.write_all(&s.into_bytes())?;
	progress.inc_and_draw(&bar, 1)
    }
    Ok(())
}
