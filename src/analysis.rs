use crate::GenerationStrategy;
use anyhow::{Context, Result};
use keycat::{
    analysis::{Analyzer, MetricData as KcMetricData, NstrokeData, NstrokeIndex},
    layout::LayoutTotals,
    Corpus, CorpusChar, Layout, Swap,
};
use keymeow::{LayoutData, MetricContext, MetricData};
use linya::Progress;
use rand::prelude::*;
use std::{fs::File, io::Write, iter};
use std::{fs::OpenOptions, io::LineWriter, sync::Mutex};

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
    let strokes: Vec<NstrokeData> = md
        .strokes
        .into_iter()
        .filter(|ns| ns.amounts.iter().any(|amt| metrics.contains(&amt.metric)))
        .collect();
    let mut position_strokes: Vec<Vec<NstrokeIndex>> = vec![vec![]; md.position_strokes[0].len()];
    for (i, stroke) in strokes.iter().map(|s| &s.nstroke).enumerate() {
        for pos in stroke.to_vec() {
            position_strokes[pos].push(i);
        }
    }
    KcMetricData {
        strokes,
        position_strokes,
        ..md
    }
}

fn layout_from_charset(corpus: &Corpus, metric_data: &MetricData, char_set: &str) -> Layout {
    let core_matrix: Vec<CorpusChar> = char_set.chars().map(|c| corpus.corpus_char(c)).collect();
    let matrix = core_matrix
        .iter()
        .chain(iter::repeat(&0usize).take(
            metric_data.keyboard.keys.map.iter().flatten().count()
                + metric_data.keyboard.combos.len()
                - core_matrix.len(),
        ))
        .copied()
        .collect();
    Layout { matrix }
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
    let layout = layout_from_charset(&corpus, &metric_data, &char_set);

    let totals = layout.totals(&corpus);

    let data = filter_metrics(kc_metric_data(metric_data, layout.matrix.len()), &metrics);
    let analyzer = Analyzer::from(data, corpus);

    let file = File::create("data/data.csv").context("couldn't create data file")?;
    let mut writer = LineWriter::new(file);

    for m in &metric_names {
        write!(writer, "{m},")?;
    }
    writeln!(writer)?;
    let progress = Mutex::new(Progress::new());
    let bar = progress.lock().unwrap().bar(count.try_into()?, "Analyzing");

    let threads: u64 = 64;
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                let count = &count.clone();
                let mut stats = analyzer.calc_stats(&layout);
                let mut layout = layout.clone();
                let mut rng = thread_rng();
                let file = OpenOptions::new()
                    .create(false)
                    .append(true)
                    .open("data/data.csv")
                    .unwrap();
                let mut writer = LineWriter::new(file);
                for _ in 0..count / threads {
                    layout.matrix.shuffle(&mut rng);
                    stats.iter_mut().for_each(|x| *x = 0.0);
                    analyzer.recalc_stats(&mut stats, &layout);
                    let mut s = String::new();
                    for m in &metrics {
                        let percent =
                            totals.percentage(stats[*m].into(), analyzer.data.metrics[*m]);
                        s.push_str(&percent.to_string());
                        s.push(',');
                    }
                    s.push('\n');
                    writer.write_all(&s.into_bytes()).unwrap();
                    progress.lock().unwrap().inc_and_draw(&bar, 1);
                }
            });
        }
    });

    Ok(())
}

struct OptimizationContext {
    layout: Layout,
    analyzer: Analyzer,
    totals: LayoutTotals,
    possible_swaps: Vec<Swap>,
    metric: usize,
}

fn greedy_neighbor_optimization(
    OptimizationContext {
        layout,
        analyzer,
        totals,
        possible_swaps,
        metric,
    }: &OptimizationContext,
) -> (u32, f32, Layout) {
    let mut rng = thread_rng();
    let mut layout = layout.clone();
    layout.matrix.shuffle(&mut rng);
    let stats = analyzer.calc_stats(&layout);
    let mut diff = vec![0.0; stats.len()];

    let mut i = 0;
    loop {
        let mut best_diff = 0.0;
        let mut best_swap = &possible_swaps[0];
        for swap in possible_swaps {
            diff.iter_mut().for_each(|x| *x = 0.0);
            analyzer.swap_diff(&mut diff, &layout, swap);
            if diff[*metric] < best_diff {
                best_swap = swap;
                best_diff = diff[*metric];
            }
        }
        if best_diff < 0.0 {
            layout.swap(best_swap);
            i += 1;
        } else {
            break;
        }
    }
    let percent = totals.percentage(
        analyzer.calc_stats(&layout)[*metric].into(),
        analyzer.data.metrics[*metric],
    );
    (i, percent, layout)
}

fn greedy_naive_optimization(
    OptimizationContext {
        layout,
        analyzer,
        totals,
        possible_swaps,
        metric,
    }: &OptimizationContext,
) -> (u32, f32, Layout) {
    let mut rng = thread_rng();
    let mut layout = layout.clone();
    layout.matrix.shuffle(&mut rng);
    let stats = analyzer.calc_stats(&layout);
    let mut diff = vec![0.0; stats.len()];

    let mut swap_i = 0;
    for i in 0..5000 {
        let swap = possible_swaps.choose(&mut rng).unwrap();
        diff.iter_mut().for_each(|x| *x = 0.0);
        analyzer.swap_diff(&mut diff, &layout, swap);
        if diff[*metric] < 0.0 {
            layout.swap(swap);
            swap_i = i;
        }
    }
    let percent = totals.percentage(
        analyzer.calc_stats(&layout)[*metric].into(),
        analyzer.data.metrics[*metric],
    );
    (swap_i, percent, layout)
}

fn simulated_annealing(
    OptimizationContext {
        layout,
        analyzer,
        totals,
        possible_swaps,
        metric,
    }: &OptimizationContext,
) -> (u32, f32, Layout) {
    let mut rng = thread_rng();
    let mut layout = layout.clone();
    layout.matrix.shuffle(&mut rng);
    let stats = analyzer.calc_stats(&layout);
    let mut diff = vec![0.0; stats.len()];

    let mut temp = 0.5;
    let iterations = 20_000;
    let dec: f32 = temp / iterations as f32;
    for _ in 0..iterations {
        temp -= dec;
        let swap = possible_swaps.choose(&mut rng).unwrap();
        diff.iter_mut().for_each(|x| *x = 0.0);
        analyzer.swap_diff(&mut diff, &layout, swap);
        if diff[*metric] < 0.0 || rng.gen::<f32>() < temp {
            layout.swap(swap);
        }
    }
    let percent = totals.percentage(
        analyzer.calc_stats(&layout)[*metric].into(),
        analyzer.data.metrics[*metric],
    );
    (iterations, percent, layout)
}

pub fn output_generation(
    metric: &str,
    metric_data: keymeow::MetricData,
    corpus: Corpus,
    char_set: &str,
    strategy: &GenerationStrategy,
    runs: u64,
) -> Result<()> {
    let metric: usize = get_metric(&metric, &metric_data).context("invalid metric")?;
    let layout = layout_from_charset(&corpus, &metric_data, &char_set);

    let totals = layout.totals(&corpus);

    let data = filter_metrics(
        kc_metric_data(metric_data, layout.matrix.len()),
        &vec![metric],
    );
    let analyzer = Analyzer::from(data, corpus);

    let possible_swaps: Vec<Swap> = (0..layout.matrix.len())
        .flat_map(|a| (0..layout.matrix.len()).map(move |b| Swap::new(a, b)))
        .filter(|Swap { a, b }| a != b)
        .collect();

    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "iteration\tamount\tlayout")?;

    let context = OptimizationContext {
        layout,
        analyzer,
        totals,
        possible_swaps,
        metric,
    };

    for _ in 0..runs {
        let (i, percent, result) = match strategy {
            GenerationStrategy::GreedyDeterministic => greedy_neighbor_optimization(&context),
            GenerationStrategy::GreedyNaive => greedy_naive_optimization(&context),
            GenerationStrategy::SimulatedAnnealing => simulated_annealing(&context),
        };
        let chars: String = result
            .matrix
            .iter()
            .map(|c| context.analyzer.corpus.uncorpus_unigram(*c))
            .map(|c| match c {
                '\0' => ' ',
                c => c,
            })
            .collect();

        writeln!(stdout, "{i}\t{percent}\t{chars}")?;
    }

    // println!("{:?}", totals.percentage(analyzer.calc_stats(&layout)[metric].into(), analyzer.data.metrics[metric]));

    Ok(())
}

pub fn stats(metric_data: MetricData, corpus: Corpus, layout: LayoutData) -> Result<()> {
    let ctx = MetricContext::new(&layout, metric_data, corpus)
        .context("could not produce metric context")?;
    let totals = ctx.layout.totals(&ctx.analyzer.corpus);

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
