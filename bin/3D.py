#!/usr/bin/env -S python -m pipx run

# SPDX-FileCopyrightText: 2024 antlers <antlers@illucid.net>
# SPDX-License-Identifier: GPL-3.0-or-later

# /// script
# requires-python = ">=3.13"
# dependencies = [
#   "docopt",
#   "matplotlib",
#   "numpy",
#   "pandas",
#   "scipy"
# ]
# ///

"""Create a heatmap of rows from FILE on an X by Y by Z grid.

FILE should be a TSV.
X, Y, and Z should specify the labels of three numerical columns.

Usage:
  3D.py [options] FILE

  -x X               X axis column label. [default: sfb]
  -y Y               Y axis column label. [default: sfs]
  -z Z               Z axis column label. [default: m3roll]
  -t, --title TITLE  Set graph title
  -o, --out OUT      Output file. [default: 3D.gif]
  -S, --score        Color by mean score instead of density.
  -A, --animate      Rotate azimuth.
  --hide-best        Don't label "best" layout.
"""

from docopt import docopt

import numpy as np
import pandas as pd
import scipy.stats as stats
import matplotlib.pyplot as plt
import multiprocessing as mp

from matplotlib import animation
from matplotlib.colors import LogNorm

from heatmap import HeatmapContext

def process_bin(bin_indices, data, x_centers, y_centers, z_centers, x, y, z, score_threshold):
    """Process a single bin, returning median score if below threshold."""
    x_bin, y_bin, z_bin = bin_indices
    x_range = (x_centers[x_bin] - (x_centers[1] - x_centers[0]) / 2, x_centers[x_bin] + (x_centers[1] - x_centers[0]) / 2)
    y_range = (y_centers[y_bin] - (y_centers[1] - y_centers[0]) / 2, y_centers[y_bin] + (y_centers[1] - y_centers[0]) / 2)
    z_range = (z_centers[z_bin] - (z_centers[1] - z_centers[0]) / 2, z_centers[z_bin] + (z_centers[1] - z_centers[0]) / 2)

    bin_points = data[(x >= x_range[0]) & (x < x_range[1]) &
                      (y >= y_range[0]) & (y < y_range[1]) &
                      (z >= z_range[0]) & (z < z_range[1])]

    if not bin_points.empty:
      median_score = bin_points['score'].median()
      if median_score <= score_threshold:
        return median_score, (x_bin, y_bin, z_bin)
    return None

class Histogram3D(HeatmapContext):
  def __init__(self, args):
    super().__init__(args, 3)
    if self.score:
      self.density_threshold_percentile=95
    else:
      self.density_threshold_percentile=90
    self.jitter_strength=0.5
    self.args = args

  def bin_data(self, num_bins=40):
    "Bin the data into a 40^3 grid based on min and max values."
    x_min, x_max = self.x.min(), self.x.max()
    y_min, y_max = self.y.min(), self.y.max()
    z_min, z_max = self.z.min(), self.z.max()

    x_bins = np.linspace(x_min, x_max, num_bins + 1)
    y_bins = np.linspace(y_min, y_max, num_bins + 1)
    z_bins = np.linspace(z_min, z_max, num_bins + 1)

    hist, edges = np.histogramdd(
      sample=[self.x, self.y, self.z],
      bins=[x_bins, y_bins, z_bins])

    x_centers = 0.5 * (edges[0][:-1] + edges[0][1:])
    y_centers = 0.5 * (edges[1][:-1] + edges[1][1:])
    z_centers = 0.5 * (edges[2][:-1] + edges[2][1:])

    return hist, x_centers, y_centers, z_centers

  def plot(self) -> tuple:
    hist, x_centers, y_centers, z_centers = self.bin_data(num_bins=40)

    if self.score:
      # Get bins where the score is above the threshold
      score_threshold = np.percentile(self.data['score'], self.density_threshold_percentile)
      with mp.Pool(mp.cpu_count()) as pool:
        results = pool.starmap(process_bin, [(bin_indices, self.data, x_centers, y_centers, z_centers, self.x, self.y, self.z, score_threshold) for bin_indices in np.ndindex(hist.shape)])

      color_values = [res[0] for res in results if res]
      non_empty_bins = [res[1] for res in results if res]
      non_empty_bins = np.array(non_empty_bins).T

      cmap = 'inferno_r'
    else:
      # Filter based on density percentiles (non-multiprocessed)
      density_threshold = np.percentile(hist.flatten(), self.density_threshold_percentile)
      non_empty_bins = np.where(hist >= density_threshold)
      color_values = hist[non_empty_bins]
      norm = LogNorm()
      cmap = self.cmap

    # Jitter positions of bins to avoid perfect alignment
    for d in self.dims:
      setattr(self, 'jitter_' + d, np.random.uniform(-self.jitter_strength, self.jitter_strength, size=len(non_empty_bins[0])))

    # Scatter the non-empty bins with color_values based on selected color option
    plt.style.use('dark_background')
    self.fig = plt.figure(figsize=(12,8))
    self.ax = self.fig.add_subplot(projection='3d')

    # Customize plot
    self.ax.set_proj_type('persp', focal_length=0.2)
    self.set_axes_labels()
    for d in c.dims:
      getattr(self.ax, d + 'axis').set_pane_color((1.0, 1.0, 1.0, 0.0))
    self.ax.set_facecolor('black')
    self.fig.set_facecolor('black')
    self.ax.grid(False)

    # # XXX: Density only
    # alpha_values = np.sign(color_values) * np.log(abs(color_values) * 2)
    # alpha_values = (alpha_values - alpha_values.min()) / (alpha_values.max() - alpha_values.min())
    # alpha_values = np.clip(alpha_values, 0.5, None)

    scatter = self.ax.scatter(
      x_centers[non_empty_bins[0]] + self.jitter_x,
      y_centers[non_empty_bins[1]] + self.jitter_y,
      z_centers[non_empty_bins[2]] + self.jitter_z,
      c=color_values,
      cmap=cmap if 'cmap' in locals() else None,
      norm=norm if 'norm' in locals() else None,
      alpha=0.8,
      s=((color_values - np.mean(color_values)) / np.std(color_values)) + 2 * 6)

    self.label_known_layouts()

    cbar = c.fig.colorbar(mappable=scatter) # type: ignore[arg-type,index]

    # XXX: Covers up the cbar label :c
    # cbar.ax.set_position([0.91, 0.1, 0.03, 0.8])

    cbar.ax.invert_yaxis()
    if c.score:
      cbar.set_label("Mean score\n(brighter is lower/better)")
    else:
      cbar.set_label("Generated layout density / bin")

    if self.title:
      plt.title(self.title)
    elif self.score:
      plt.title(f'Top {self.density_threshold_percentile}th Percentile by Score')
    else:
      plt.title(f'Top {self.density_threshold_percentile}th Percentile by Density')

    if self.args['--animate']:
      def animate(i):
        self.ax.view_init(elev=10, azim=i)
        return scatter,
      ani = animation.FuncAnimation(self.fig, animate, frames=np.arange(0, 360, 2), interval=1000, blit=True)
      writer = animation.PillowWriter(fps=15, bitrate=1800)
      ani.save(self.out, writer=writer)

    else:
      plt.show()

# === Main ===
if __name__ == "__main__":
  args = docopt(__doc__)

  c = Histogram3D(args)

  # Visualize the top percentile of the data
  c.plot()
