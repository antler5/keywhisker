#!/usr/bin/env -S python -m pipx run

# SPDX-FileCopyrightText: 2024 antlers <antlers@illucid.net>
# SPDX-License-Identifier: GPL-3.0-or-later

# /// script
# requires-python = ">=3.13"
# dependencies = [
#   "docopt",
#   "numpy",
#   "matplotlib",
#   "pandas==2.2.3"
# ]
# ///

# mypy: disable-error-code="attr-defined"

"""Create a heatmap of rows from FILE on an X by Y grid.

FILE should be a TSV.
X and Y should specify the labels of two numerical columns.

Usage:
  heatmap.py [options] FILE

  -x X               X axis column label. [default: m3roll]
  -y Y               Y axis column label. [default: sfb]
  -t, --title TITLE  Graph title. [default: 2x4 w/ Thumb]
  -o, --out OUT      Output file. [default: img.svg]
  -H, --hex          Use hexbins instead of hist2d.
  -S, --score        Color by mean score instead of density.
  --hide-best        Don't label "best" layout.
"""

# === Imports ===
import sys

from abc import ABC, abstractmethod
from typing import Any, Final, Literal

from docopt import docopt

import matplotlib
import matplotlib.pyplot as plt
import pandas as pd
import numpy as np

from matplotlib.ticker import PercentFormatter
from matplotlib.colors import LogNorm
from matplotlib.lines import Line2D

# === Variables ===

# Known layouts
def row(m3roll: float, sfb: float, sfs: float) -> dict:
  return {
    'iteration': 0,
    'score': 1425000000, # a modest avg
    'm3roll': m3roll,
    'sfb': sfb,
    'sfs': sfs,
    'layout': None,
  }

known_layouts: dict = {
  # 'ardux':         row(1.32, 92.26, 90.93),
  'ardux-no-spc':  row(2.92, 54.56, 58.45),
  # 'artsey':        row(1.38, 90.90, 90.19),
  'artsey-no-spc': row(3.02, 53.16, 58.23),
  'caret':         row(5.72, 52.58, 50.16),
  'caret-no-spc':  row(9.92, 34.25, 50.20),
  'taipo':         row(9.69, 31.11, 33.85),
  'taipo-no-spc':  row(4.31, 49.18, 51.38),
  '1-taurine':     row(14.41, 28, 38.73),
  '2':             row(18.03, 27.88, 42.89),
  '3':             row(12.96, 28.09, 36.39),
  '4':             row(9.98, 28.54, 37.18),
  '5':             row(15.015007, 30.14812, 37.49303),
  '6':             row(15.273136,32.06765,36.43247),
  '7':             row(12.924261,24.590355,43.340397)
}

# === Helpers ===
def maybe_upper(s: str) -> str:
  "Uppercase string S if it doesn't have any numbers in it."
  return s.upper() if s.isalpha() else s

class HeatmapContext(ABC):
  def __init__(self, args, dim: int) -> None:
    for k, v in args.items():
      setattr(self, k.lstrip('-').lower().replace('-', '_'), v)

    self.load_data()

    self.dim = dim
    self.dims = ['x','y','z'][:dim]

    for d in self.dims:
      label = getattr(self, d)
      data = self.data[label]
      setattr(self, d + 'label', label)
      setattr(self, d, data)

  def load_data(self) -> None:
    "Read file from args and add known layouts."
    data = pd.read_csv(self.file, sep='\t')
    print(data)

    # Concat known layouts into data, setting any missing columns to NaN.
    if not self.hide_best:
      known_layouts['0-best'] = data.loc[data['score'].idxmin()]
    df = pd.DataFrame(known_layouts.values())
    for col in data.columns.difference(df.columns):
      df[col] = np.nan
    data = pd.concat([data, df], ignore_index=True)

    self.data = data

  def set_axes_labels(self) -> None:
    "Set and style axis labels from args."
    for d in self.dims:
      set_label = getattr(self.ax, 'set_' + d + 'label')
      set_label(maybe_upper(getattr(self, d + 'label')))

      set_major_formatter = getattr(self.ax, d + 'axis').set_major_formatter
      set_major_formatter(PercentFormatter(decimals=0))

  def label_known_layouts(self) -> None:
    "Label known layouts on Axes AX."
    for layout, metrics in known_layouts.items():
      color = 'red' if layout[0].isdigit() else 'orange'
      pos: list[float] = []

      for d in self.dims:
        pos += [metrics[getattr(self, d + 'label')]]

      self.ax.scatter(
        *pos, # type: ignore[arg-type,misc]
        label=layout,
        marker='o',
        s=25, color=color,
        edgecolor='#555')

      self.ax.text( # type: ignore[call-arg]
        *(pos[0]+0.35, *pos[1:]), layout, # type: ignore[arg-type]
        va='center', color='black', fontsize=8,
        bbox=dict(facecolor='#D9D9D9', edgecolor='#555', boxstyle='round'))

  @abstractmethod
  def plot(self) -> tuple:
    pass

class Histogram2D(HeatmapContext):
  def __init__(self, args):
    super().__init__(args, 2)

  def plot(self) -> tuple:
    hist: Any

    if self.score:
      cmap = 'inferno_r'

    if self.hex:
      if self.score:
        C = self.data['score']
        reduce_C_function = np.mean
      else:
        bins: Literal['log'] = 'log'
        mincnt = 1

      hist = self.ax.hexbin(
        self.x, self.y,
        gridsize=(50,30), linewidths=0,
        C=C if 'C' in locals() else None,
        bins=bins if 'bins' in locals() else None,
        cmap=cmap if 'cmap' in locals() else None,
        mincnt=mincnt if 'mincnt' in locals() else None,
        reduce_C_function=reduce_C_function if 'reduce_C_function' in locals() else None) # type: ignore[arg-type]
      return (hist, hist)

    else:
      if self.score:
        # Couldn't color by score by setting hist2d's `weights`, so I'll do the
        # math myself and call `pcolormesh`.

        # Create bins
        x_bins, y_bins = (64, 64)
        x_edges = np.linspace(self.x.min(), self.x.max(), x_bins + 1)
        y_edges = np.linspace(self.y.min(), self.y.max(), y_bins + 1)
        hist , xedges, yedges = np.histogram2d(self.x, self.y, bins=[x_edges, y_edges]) # type: ignore[list-item]

        # Calculate means
        bin_means = np.zeros_like(hist)
        for i in range(x_bins):
          for j in range(y_bins):
            # Get indexes of points in bin
            mask = ((self.x >= xedges[i]) & (self.x < xedges[i + 1]) &
                    (self.y >= yedges[j]) & (self.y < yedges[j + 1]))
            bin_means[i, j] = self.data.loc[mask, 'score'].mean() if mask.any() else np.nan

        hist = plt.pcolormesh(x_edges, y_edges, bin_means.T, cmap=cmap)
        return (hist, hist)
      else:
        hist = self.ax.hist2d(self.x, self.y, bins=64, norm=LogNorm())
        return (hist, hist[3])

# === Main ===
if __name__ == "__main__":
  args = docopt(__doc__)
  c = Histogram2D(args)

  c.fig: plt.Figure
  c.ax: plt.Axes
  fig, ax = plt.subplots(tight_layout=True)
  setattr(c, 'fig', fig)
  setattr(c, 'ax', ax)
  c.fig.patch.set_facecolor('#D9D9D9')
  ax.set_facecolor("black")

  hist, mappable = c.plot()

  c.set_axes_labels()
  c.label_known_layouts()

  # Colorbar
  cbar = c.fig.colorbar(mappable=mappable, ax=c.ax) # type: ignore[arg-type,index]

  if c.score:
    cbar.ax.invert_yaxis()
    cbar.set_label("Mean score / cell\n(brighter is better)")
  else:
    cbar.set_label("Generated layout density\n(per ~1/64²)")

  # Export
  plt.title(c.title + "\n" + f'({len(c.data):,} samples)')
  plt.savefig(c.out, dpi=180)

  print("Created", c.out)
