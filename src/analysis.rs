use crate::GenerationStrategy;
use crate::ddako::simulated_annealing as ddako_sa;

use anyhow::{Context, Result};
use keycat::{
    analysis::{Analyzer, MetricData as KcMetricData, NstrokeData, NstrokeIndex},
    Corpus, CorpusChar, Layout, NgramType, Swap,
};
use keymeow::{LayoutData, MetricContext, MetricData};
use linya::Progress;
use rand::prelude::*;
use rand::distributions::{Alphanumeric, DistString};
use std::fmt::Write as StringWrite;
use std::path::Path;
use std::{fs::File, io::Write, iter};
use std::{fs::OpenOptions, io::LineWriter, sync::Mutex};

use std::time::Instant;
use std::time::Duration;

use indexmap::IndexMap;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Row, Table, TableState},
    Terminal,
};

fn print_hashmap(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    mut table_state: &mut TableState,
    map: &indexmap::IndexMap<&str, String>,
) {
    if atty::is(atty::Stream::Stdout) {
        terminal.clear().unwrap();
        terminal.draw(|f| {
            let chunks = ratatui::layout::Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(100)].as_ref())
                .split(f.area());

            let table = Table::new(
                map.iter().map(|(key, value)| {
                    if *key == "Initial Temp Stats" {
                        Row::new(vec![
                            Span::styled(key.to_string(), Style::default().fg(Color::Gray)),
                            Span::styled(value.to_string(), Style::default().fg(Color::White)),
                        ])
                    } else {
                        Row::new(vec![
                            Span::styled(key.to_string(), Style::default().fg(Color::Yellow)),
                            Span::styled(value.to_string(), Style::default().fg(Color::White)),
                        ])
                    }
                }),
                &[
                    Constraint::Percentage(30),
                    Constraint::Percentage(70),
                ]
            )
            .header(Row::new(vec![
                Span::styled("Key", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled("Value", Style::default().add_modifier(Modifier::BOLD)),
            ]))
            .block(Block::default().borders(Borders::ALL).title("Keywhisker"));

            if table_state.selected().is_none() {
                table_state.select(Some(0));
            }

            f.render_stateful_widget(table, chunks[0], &mut table_state);
        })
        .unwrap();
    }
}

fn create_rate_tracker<'a>(
    mut terminal: &'a mut Terminal<CrosstermBackend<std::io::Stdout>>,
    mut table_state: &'a mut TableState,
) -> impl FnMut(&mut IndexMap<&str, String>) + use<'a> {
    let mut last_print = Instant::now();
    let mut last_call = Instant::now();
    let mut calls = 0u64;
    let mut min_interval = Duration::from_secs(u64::MAX);
    let mut max_interval = Duration::from_secs(0);

    move |rt_stats: &mut IndexMap<&str, String>| {
        let now = Instant::now();
        let interval = now.duration_since(last_call);
        min_interval = min_interval.min(interval);
        max_interval = max_interval.max(interval);
        last_call = now;
        calls += 1;

        if now.duration_since(last_print) >= Duration::from_secs(3) {
            let elapsed = now.duration_since(last_print);
            let rate = calls as f64 / elapsed.as_secs_f64();
            for (label, stat) in &mut *rt_stats {
                match *label {
                    "Evaluation Rate" => *stat = format!("{:.5} swaps/second", rate),
                    "Min/Max Interval" => *stat = format!("{:?} \t/ {:?}", min_interval, max_interval),
                    _ => (),
                }
            }
            print_hashmap(&mut terminal, &mut table_state, &rt_stats);

            // Reset stats
            calls = 0;
            last_print = now;
            min_interval = Duration::from_secs(u64::MAX);
            max_interval = Duration::from_secs(0);
        }
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
    Layout(matrix)
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
    let layout = layout_from_charset(&corpus, &metric_data, char_set);

    let totals = layout.totals(&corpus);

    let data = filter_metrics(kc_metric_data(metric_data, layout.0.len()), &metrics);
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
                    layout.0.shuffle(&mut rng);
                    stats.iter_mut().for_each(|x| *x = 0.0);
                    analyzer.recalc_stats(&mut stats, &layout);
                    let mut s = String::new();
                    for m in &metrics {
                        let percent = totals.percentage(stats[*m], analyzer.data.metrics[*m]);
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
    possible_swaps: Vec<Swap>,
    evaluator: Evaluator,
    pin: usize,
}

pub struct Evaluator {
    metrics: Vec<(usize, f32)>,
}

impl From<Vec<(usize, i16)>> for Evaluator {
    fn from(metrics: Vec<(usize, i16)>) -> Self {
        let sum: f32 = metrics.iter().map(|(_, x)| *x as f32).sum();
        Self {
            metrics: metrics.iter().map(|(m, x)| (*m, *x as f32 / sum)).collect(),
        }
    }
}

impl Evaluator {
    pub fn eval(&self, stats: &[f32]) -> f32 {
        self.metrics.iter().map(|(m, x)| x * stats[*m]).sum()
    }
}

fn greedy_neighbor_optimization(
    OptimizationContext {
        layout,
        analyzer,
        possible_swaps,
        evaluator,
        pin,
    }: &OptimizationContext,
) -> (u32, f32, Vec<f32>, Layout) {
    let mut rng = thread_rng();
    let mut layout = layout.clone();

    // Shuffle without moving pinned keys
    layout.0[*pin..].shuffle(&mut rng);

    let stats = analyzer.calc_stats(&layout);
    let mut diff = vec![0.0; stats.len()];

    let mut i = 0;
    loop {
        let mut best_diff = 0.0;
        let mut best_swap = &possible_swaps[0];
        for swap in possible_swaps {
            evaluator.metrics.iter().for_each(|(index, _)| diff[*index] = 0.0);
            diff.iter_mut().for_each(|x| *x = 0.0);
            analyzer.swap_diff(&mut diff, &layout, swap);
            let score = evaluator.eval(&diff);
            if score < best_diff {
                best_swap = swap;
                best_diff = score;
            }
        }
        if best_diff+0.000001 < 0.0 {
            layout.swap(best_swap);
            i += 1;
        } else {
            break;
        }
    }
    let stats = analyzer.calc_stats(&layout);
    let score = evaluator.eval(&stats);
    (i, score, stats, layout)
}

fn greedy_naive_optimization(
    OptimizationContext {
        layout,
        analyzer,
        possible_swaps,
        evaluator,
        pin,
    }: &OptimizationContext,
) -> (u32, f32, Vec<f32>, Layout) {
    let mut rng = thread_rng();
    let mut layout = layout.clone();

    // Shuffle without moving pinned keys
    layout.0[*pin..].shuffle(&mut rng);

    let stats = analyzer.calc_stats(&layout);
    let mut diff = vec![0.0; stats.len()];

    let mut swap_i = 0;
    for i in 0..5000 {
        let swap = possible_swaps.choose(&mut rng).unwrap();
        diff.iter_mut().for_each(|x| *x = 0.0);
        analyzer.swap_diff(&mut diff, &layout, swap);
        let score = evaluator.eval(&diff);
        if score < 0.0 {
            layout.swap(swap);
            swap_i = i;
        }
    }
    let stats = analyzer.calc_stats(&layout);
    let score = evaluator.eval(&stats);
    (swap_i, score, stats, layout)
}

fn simulated_annealing(
    OptimizationContext {
        layout,
        analyzer,
        possible_swaps,
        evaluator,
        pin,
    }: &OptimizationContext,
) -> (u32, f32, Vec<f32>, Layout) {
    let mut rng = thread_rng();
    let mut layout = layout.clone();

    // Shuffle without moving pinned keys
    layout.0[*pin..].shuffle(&mut rng);

    let stats = analyzer.calc_stats(&layout);
    let mut diff = vec![0.0; stats.len()];

    let mut temp = 0.5;
    let iterations = 1_000_000;
    let dec: f32 = temp / iterations as f32;
    for _ in 0..iterations {
        temp -= dec;
        let swap = possible_swaps.choose(&mut rng).unwrap();
        diff.iter_mut().for_each(|x| *x = 0.0);
        analyzer.swap_diff(&mut diff, &layout, swap);
        let score = evaluator.eval(&diff);
        if score < 0.0 || rng.gen::<f32>() < temp {
            layout.swap(swap);
        }
    }
    let stats = analyzer.calc_stats(&layout);
    let score = evaluator.eval(&stats);
    (iterations, score, stats, layout)
}

fn ddako_simulated_annealing(
    OptimizationContext {
        layout,
        analyzer,
        possible_swaps,
        evaluator,
        pin: _pin,
    }: &OptimizationContext,
) -> (u32, f32, Vec<f32>, Layout) {
    let backend = CrosstermBackend::new(std::io::stdout());
    let mut terminal = Terminal::new(backend).unwrap();

    let mut table_state = TableState::default();
    let mut rt = create_rate_tracker(&mut terminal, &mut table_state);

    let mut sa = ddako_sa::SimulatedAnnealing::new(
        possible_swaps,
        layout,
        analyzer,
        evaluator,
        0.9,
        5.0,
        1.0,
        10.0,
        None,
        &mut rt,
    );

    sa.optimize(possible_swaps.len())
}

pub fn output_generation(
    metrics: &[(String, i16)],
    metric_data: keymeow::MetricData,
    corpus: Corpus,
    char_set: &str,
    strategy: &GenerationStrategy,
    pin: usize,
    runs: u64,
    use_stdout: bool,
) -> Result<()> {
    let metric_weights: Result<Vec<_>> = metrics
        .iter()
        .map(|(name, x)| {
            let metric =
                get_metric(name, &metric_data).with_context(|| format!("invalid metric {name}"));
            match metric {
                Ok(m) => Ok((m, *x)),
                Err(e) => Err(e),
            }
        })
        .collect();
    let metric_weights = metric_weights?;
    let evaluator = Evaluator::from(metric_weights.clone());
    let layout = layout_from_charset(&corpus, &metric_data, char_set);

    let data = filter_metrics(
        kc_metric_data(metric_data, layout.0.len()),
        &metric_weights
            .iter()
            .map(|(m, _)| *m)
            .collect::<Vec<usize>>(),
    );
    let analyzer = Analyzer::from(data, corpus);

    // Swap without moving pinned keys
    let possible_swaps: Vec<Swap> = (0..layout.0.len())
        .flat_map(|a| (0..layout.0.len()).map(move |b| Swap::new(a, b)))
        .filter(|Swap { a, b }| a != b && *a > pin && *b > pin)
        .collect();

    let output: &mut dyn Write = if use_stdout {
        &mut std::io::stdout().lock()
    } else {
        let random_string = Alphanumeric.sample_string(&mut rand::thread_rng(), 8);
        let name: String = [format!("generate_{:?}_{}", &strategy, random_string)]
            .into_iter()
            .chain([".tsv".to_string()])
            .collect();
        &mut File::create_new(Path::new("generations").join(&name))?
    };
    let mut s: String = "iteration\tscore\t".into();
    metrics.iter().for_each(|(m, _)| {
        s.push_str(m);
        s.push('\t');
    });
    s.push_str("layout");

    writeln!(output, "{}", s)?;

    let context = OptimizationContext {
        layout,
        analyzer,
        possible_swaps,
        evaluator,
        pin,
    };

    let totals = context.layout.totals(&context.analyzer.corpus);

    for _ in 0..runs {
        let (i, score, stats, result) = match strategy {
            GenerationStrategy::GreedyDeterministic => greedy_neighbor_optimization(&context),
            GenerationStrategy::GreedyNaive => greedy_naive_optimization(&context),
            GenerationStrategy::SimulatedAnnealing => simulated_annealing(&context),
            GenerationStrategy::DDAKOSimulatedAnnealing => ddako_simulated_annealing(&context),
        };
        let chars: String = result
            .0
            .iter()
            .map(|c| context.analyzer.corpus.uncorpus_unigram(*c))
            .map(|c| match c {
                '\0' => '�',
                c => c,
            })
            .collect();
        let mut values = String::new();
        for (m, _) in metric_weights.iter() {
            values.push_str(&format!(
                "{}\t",
                totals.percentage(stats[*m], context.analyzer.data.metrics[*m])
            ))
        }

        writeln!(output, "{i}\t{score}\t{values}{chars}")?;
    }

    // println!("{:?}", totals.percentage(analyzer.calc_stats(&layout)[metric].into(), analyzer.data.metrics[metric]));

    Ok(())
}

pub fn stats(metric_data: MetricData, corpus: Corpus, layouts: Vec<LayoutData>) -> Result<()> {
    let ctx = MetricContext::new(
        layouts
            .first()
            .context("need at least one layout to show stats for")?,
        metric_data,
        corpus,
    )
    .context("could not produce metric context")?;
    let totals = ctx.layout.totals(&ctx.analyzer.corpus);

    let stat_lists: Vec<Vec<f32>> = layouts
        .iter()
        .map(|l| {
            let matrix = MetricContext::layout_matrix(l, &ctx.keyboard, &ctx.analyzer.corpus)
                .with_context(|| format!("layout {} incompatible with keyboard", l.name))
                .unwrap();
            ctx.analyzer.calc_stats(&matrix)
        })
        .collect();
    let max: usize = ctx.metrics.iter().map(|m| m.name.len()).max().unwrap();
    let name_lengths: Vec<usize> = layouts.iter().map(|l| l.name.len()).collect();

    let labels = layouts
        .iter()
        .fold(str::repeat(" ", max + 1), |mut output, l| {
            let _ = write!(
                output,
                "{}{}",
                l.name,
                str::repeat(" ", 4 + 7_usize.saturating_sub(l.name.len()))
            );
            output
        });

    println!("{labels}");

    for i in 0..ctx.metrics.len() {
        let name = &ctx.metrics[i].name;
        let percentages: String =
            stat_lists
                .iter()
                .enumerate()
                .fold(String::new(), |mut output, (col, s)| {
                    let pc = totals.percentage(s[i], ctx.metrics[i].ngram_type);
                    let len = match pc {
                        x if x < 10. => 5,
                        x if x < 100. => 6,
                        _ => 7,
                    };
                    let name_spacing = 4 + 7_usize.saturating_sub(name_lengths[col]);
                    let _ = write!(
                        output,
                        "{:.2}%{}",
                        pc,
                        str::repeat(" ", name_lengths[col] + name_spacing - len)
                    );
                    output
                });
        println!(
            "{}{}{}",
            name,
            str::repeat(" ", 1 + max - name.len()),
            percentages
        )
    }

    Ok(())
}

pub fn combos(metric_data: MetricData, corpus: Corpus, layout: LayoutData) -> Result<()> {
    let mut ctx = MetricContext::new(&layout, metric_data, corpus)
        .context("could not produce metric context")?;
    let totals = ctx.layout.totals(&ctx.analyzer.corpus);
    // let stats = ctx.analyzer.calc_stats(&ctx.layout);

    let kb_size = ctx.keyboard.keys.map.iter().flatten().count();
    ctx.keyboard.process_combo_indexes();

    let mut i = 0;
    for (idx, combo) in ctx.keyboard.combo_indexes.iter().enumerate() {
        let combo_text: String = combo
            .iter()
            .take(3)
            .filter_map(|i| {
                let cc = ctx.layout.0[*i];
                if cc == 0 {
                    return None;
                }
                let c = ctx.analyzer.corpus.uncorpus_unigram(cc);
                match c {
                    ' ' => Some('␣'),
                    _ => Some(c),
                }
            })
            .collect();
        let key = ctx.layout.0[kb_size + idx];
        let output = match key {
            0 => ' ',
            _ => ctx.analyzer.corpus.uncorpus_unigram(key),
        };
        let spacing = str::repeat(" ", 4 - combo.len());
        let freq = totals.percentage(ctx.analyzer.corpus.chars[key] as f32, NgramType::Bigram);
        let freq_text = match output {
            ' ' => String::from("      "),
            _ => format!("({:.1}%)", freq),
        };
        print!("{combo_text}{spacing}{output} {freq_text}\t");
        i += 1;
        if i % 4 == 0 {
            println!();
        }
    }
    println!();

    Ok(())
}
