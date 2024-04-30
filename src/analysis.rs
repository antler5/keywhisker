use anyhow::{Context, Result};
use keycat::{
    analysis::{Analyzer, MetricData as KcMetricData, NstrokeData, NstrokeIndex},
    Corpus, CorpusChar, Layout, Swap,
};
use keymeow::{LayoutData, MetricContext, MetricData};
use linya::Progress;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::{fs::File, io::Write, iter, path::Path};
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
        .chain(
            iter::repeat(&0usize).take(
                metric_data
                    .keyboard
                    .keys
                    .map
                    .iter()
                    .map(|v| v.len())
                    .sum::<usize>()
                    - core_matrix.len(),
            ),
        )
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

pub fn output_greedy(
    metric: &str,
    metric_data: keymeow::MetricData,
    corpus: Corpus,
    char_set: &str,
    iterations: u64,
) -> Result<()> {
    let metric: usize = get_metric(&metric, &metric_data).context("invalid metric")?;
    let mut layout = layout_from_charset(&corpus, &metric_data, &char_set);

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

    let mut rng = thread_rng();

    let file = File::create(Path::new("data").join("greedy_neighbor_runs.csv"))
        .context("couldn't create data file")?;
    let mut writer = LineWriter::new(file);
    writeln!(writer, "iteration,amount")?;

    for n in 0..100_000 {
        if n % 1000 == 0 {
            println!("{n}");
        }
        
        layout.matrix.shuffle(&mut rng);
        let stats = analyzer.calc_stats(&layout);
        let mut diff = vec![0.0; stats.len()];

	let mut i = 0;
	loop {
	    let mut best_diff = 0.0;
	    let mut best_swap = &possible_swaps[0];
	    for swap in &possible_swaps {
		diff.iter_mut().for_each(|x| *x = 0.0);
		analyzer.swap_diff(&mut diff, &layout, swap);
		if diff[metric] < best_diff {
                    best_swap = swap;
		    best_diff = diff[metric];
                }	
	    }
	    if best_diff < 0.0 {
		layout.swap(best_swap);
		i += 1;
	    } else {
		break;
	    }
        };
	let percent =
            totals.percentage(analyzer.calc_stats(&layout)[metric].into(), analyzer.data.metrics[metric]);
	writeln!(writer, "{i}, {percent:.2}")?;
    }

    // let output: Vec<_> = layout.matrix.iter().map(|i| analyzer.corpus.uncorpus_unigram(*i)).collect();
    // for row in 0..3 {
    // 	for col in 0..5 {
    // 	    print!("{} ", output[col*3 + row]);
    // 	}
    // 	print!(" ");
    // 	for col in 5..10 {
    // 	    print!("{} ", output[col*3 + row]);
    // 	}
    // 	println!();
    // }

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
