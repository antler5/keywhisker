use core::clone::Clone;
use rand::prelude::*;
use rand::Rng;
use std::f32::consts::E;

use crate::analysis::Evaluator;
use keycat::analysis::Analyzer;
use keycat::{Layout, Swap};

pub struct SimulatedAnnealing<'a> {
    possible_swaps: Vec<Swap>,
    layout: Layout,
    analyzer: &'a Analyzer,
    stats: Vec<f32>,
    diff: Vec<f32>,
    evaluator: &'a Evaluator,
    cooling_rate: f32,
    cooling_interval: f32,
    cooling_interval_min: f32,
    cooling_interval_max: f32,
    max_iterations: Option<u32>,
    fitness: f32,
    temp: Option<f32>,
    stopping_point: Option<usize>,
}

impl<'a> SimulatedAnnealing<'a> {
    pub fn new(
        possible_swaps: &Vec<Swap>,
        layout: &Layout,
        analyzer: &'a Analyzer,
        evaluator: &'a Evaluator,
        cooling_rate: f32,
        cooling_interval: f32,
        cooling_interval_min: f32,
        cooling_interval_max: f32,
        max_iterations: Option<u32>,
    ) -> Self {
        let stats = analyzer.calc_stats(layout);
        let initial_fitness = evaluator.eval(&stats);
        let len = stats.len();

        SimulatedAnnealing {
            possible_swaps: possible_swaps.to_vec(),
            layout: layout.clone(),
            analyzer,
            stats,
            diff: vec![0.0; len],
            evaluator,
            cooling_rate,
            cooling_interval,
            cooling_interval_min,
            cooling_interval_max,
            max_iterations,
            fitness: initial_fitness,
            temp: None,
            stopping_point: None,
        }
    }

    fn _evaluate_swap(&mut self, swap: &Swap) -> f32 {
        self.diff.iter_mut().for_each(|x| *x = 0.0);
        self.analyzer.swap_diff(&mut self.diff, &self.layout, swap);

        let score = self.evaluator.eval(&self.diff);

        self.fitness + score
    }

    fn evaluate_swap_slowly(&mut self, swap: &Swap) -> f32 {
        // but correctly :c
        self.layout.swap(swap);
        self.diff.iter_mut().for_each(|x| *x = 0.0);
        self.analyzer.recalc_stats(&mut self.diff, &self.layout);

        let score = self.evaluator.eval(&self.diff);

        let reverse_swap = Swap {
            a: swap.b,
            b: swap.a,
        };
        self.layout.swap(&reverse_swap);

        score
    }

    fn get_initial_temperature(&mut self, acceptance_ratio: f32, epsilon: f32) -> f32 {
        let mut tn = self.fitness;
        let mut acceptance_probability = 0.0;

        while (acceptance_probability - acceptance_ratio).abs() > epsilon {
            let mut energies = Vec::new();

            for new_swap in &self.possible_swaps.clone() {
                let new_fitness = self.evaluate_swap_slowly(new_swap);
                let delta = new_fitness - self.fitness;

                if delta > 0.001 {
                    energies.push(new_fitness);
                }
            }

            if !energies.is_empty() {
                let sum_exp: f32 = energies.iter().map(|e| E.powf(-*e / tn)).sum();

                acceptance_probability =
                    sum_exp / (energies.len() as f32 * E.powf(-self.fitness / tn));

                tn *= acceptance_probability.ln() / acceptance_ratio.ln();
            } else {
                tn *= 2.0;
            }
        }

        tn
    }

    fn get_stopping_point(&self, layout_size: usize) -> usize {
        let possible_swaps = layout_size as f32;
        let euler_mascheroni = 0.577_215_7;
        ((possible_swaps * (possible_swaps.ln() + euler_mascheroni) + 0.5).ceil()) as usize
    }

    pub fn optimize(
        &mut self,
        layout_size: usize,
    ) -> (u32, f32, Vec<f32>, Layout) {
        let mut rng = rand::thread_rng();

        if self.temp.is_none() {
            self.temp = Some(self.get_initial_temperature(0.8, 0.01));
        }
        if self.stopping_point.is_none() {
            self.stopping_point = Some(self.get_stopping_point(layout_size));
        }

        let mut best_layout = self.layout.0.clone();
        let mut best_fitness = self.fitness;
        let mut stays = 0;
        let mut iteration: u32 = 0;
        let mut last_adjustment = 0;

        let mut recent_acceptances = Vec::new();
        let mut recent_acceptance_rates = Vec::new();
        let window_size = 20;

        let mut last_improvement_iteration = 0;

        while stays < self.stopping_point.unwrap() {
            if let Some(max_iter) = max_iterations {
                if iteration >= max_iter {
                    break;
                }
            }

            for _ in 0..layout_size {
                let new_swap = self.possible_swaps.choose(&mut rng).unwrap().clone();
                let new_fitness = self.evaluate_swap_slowly(&new_swap);
                let delta = new_fitness - self.fitness;

                let mut accepted = false;
                if delta < 0.0 {
                    recent_acceptances.push(true);
                    accepted = true;
                    stays = 0;
                } else if rng.gen::<f32>() < E.powf(-delta / self.temp.unwrap()) {
                    accepted = true;
                    stays = stays.saturating_sub(1);
                } else {
                    recent_acceptances.push(false);
                    stays += 1;
                };

                if recent_acceptances.len() > window_size {
                    recent_acceptances.remove(0);
                }

                if accepted {
                    self.layout.swap(&new_swap);
                    self.stats.iter_mut().for_each(|x| *x = 0.0);
                    self.analyzer.recalc_stats(&mut self.stats, &self.layout);

                    // assert(new_fitness > 0.001)

                    self.fitness = new_fitness;

                    if self.fitness < best_fitness {
                        last_improvement_iteration = iteration;
                        best_layout = self.layout.0.clone();
                        best_fitness = self.fitness;
                    }
                }
            }

            let acceptance_rate = recent_acceptances.iter().filter(|&&x| x).count() as f32
                / recent_acceptances.len() as f32;
            recent_acceptance_rates.push(acceptance_rate);
            if recent_acceptance_rates.len() > window_size {
                recent_acceptance_rates.remove(0);
            }

            let time_since_improvement = iteration - last_improvement_iteration;

            // Cooling & Interval adjustment
            if iteration > 0 && (iteration - last_adjustment) % self.cooling_interval as u32 == 0 {
                last_adjustment = iteration;
                self.temp = Some(self.temp.unwrap() * self.cooling_rate);

                if acceptance_rate > 0.1 || self.cooling_interval > time_since_improvement as f32 {
                    self.cooling_interval =
                        (self.cooling_interval * 1.1).min(self.cooling_interval_max);
                } else {
                    self.cooling_interval =
                        (self.cooling_interval * 0.9).max(self.cooling_interval_min);
                }
            }
            iteration += 1;
        }

        let layout = Layout(best_layout);
        self.stats = self.analyzer.calc_stats(&layout);
        (iteration, best_fitness, self.stats.clone(), layout)
    }
}
