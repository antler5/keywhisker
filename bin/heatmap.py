#!/usr/bin/env -S python -m pipx run

# /// script
# requires-python = ">=3.13"
# dependencies = [
#   "docopt",
#   "numpy",
#   "matplotlib",
#   "pandas"
# ]
# ///

"""Create a heapmap of rows from FILE on an X by Y grid.

FILE should be a TSV.
X and Y should specify the labels of two numerical columns.

Usage:
  heatmap.py [options] FILE

  -x X      X axis column label. [default: m3roll]
  -y Y      Y axis column label. [default: sfb]
  -t TITLE  Graph title. [default: 2x4 w/ Thumb]
  -o OUT    Output file. [default: img.svg]
  -H        Use hexbins instead of hist2d.
  -S        Color by mean score instead of density.
"""

# === Imports ===
import sys

from typing import Any

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
    'score': sys.maxsize,
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
  'taipo-no-spc':  row(4.31, 49.18, 51.38)
}

# === Helpers ===
def maybe_upper(s: str) -> str:
  "Uppercase string S if it doesn't have any numbers in it."
  return s.upper() if s.isalpha() else s

def load_data(args: dict) -> pd.DataFrame:
  "Read file from args and add known layouts."
  data = pd.read_csv(args['FILE'], sep='\t')

  # Concat known layouts into data, setting any missing columns to NaN.
  known_layouts['interesting'] = data.loc[data['score'].idxmin()]
  df = pd.DataFrame(known_layouts.values())
  for col in data.columns.difference(df.columns):
    df[col] = np.nan
  data = pd.concat([data, df], ignore_index=True)

  return data

def set_axes_labels(args: dict, ax: matplotlib.axes._axes.Axes, dim: int) -> None:
  "Set and style axis labels from args."
  ax.set_facecolor('black')
  for i in range(0,dim):
    dim_letter = ['x','y','z'][i]

    set_label = getattr(ax, 'set_' + dim_letter + 'label')
    set_label(maybe_upper(args['-' + dim_letter]))

    set_major_formatter = getattr(ax, dim_letter + 'axis').set_major_formatter
    set_major_formatter(PercentFormatter(decimals=0))

def label_known_layouts(args: dict, ax: matplotlib.axes._axes.Axes, dim: int) -> None:
  "Label known layouts on Axes AX."
  for layout, metrics in known_layouts.items():
    color = 'red' if layout == 'interesting' else 'orange'
    pos: list[float] = []

    for i in range(0,dim):
      dim_letter = ['x','y','z'][i]
      pos += [metrics[args['-' + dim_letter]]]

    ax.scatter(
      *pos, # type: ignore[arg-type,misc]
      label=layout,
      marker='o',
      s=25, color=color,
      edgecolor='#555'
    )

    ax.annotate(
      layout,
      (pos[0]+0.35, *pos[1:]), # type: ignore[arg-type]
      va='center', color='black', fontsize=8,
      bbox=dict(facecolor='#D9D9D9', edgecolor='#555', boxstyle='round')
    )

def make_hist(args: dict, ax: matplotlib.axes._axes.Axes) -> Any:
  (x, y) = (data[args['-x']], data[args['-y']])

  if args['-S']:
    cmap = 'inferno_r'

  if args['-H']:
    if args['-S']:
      C = data['score']
      reduce_C_function = np.mean
    else:
      bins = 'log'
      mincnt = 1

    hist = ax.hexbin(
      x, y,
      gridsize=(50,30), linewidths=0,
      C=C if 'C' in locals() else None,
      bins=bins if 'bins' in locals() else None,
      cmap=cmap if 'cmap' in locals() else None,
      mincnt=mincnt if 'mincnt' in locals() else None,
      reduce_C_function=reduce_C_function if 'reduce_C_function' in locals() else None)
    return (hist, hist)

  else:

    if args['-S']:
      # Create bins
      x_bins, y_bins = (64, 64)
      x_edges = np.linspace(x.min(), x.max(), x_bins + 1)
      y_edges = np.linspace(y.min(), y.max(), y_bins + 1)
      hist, xedges, yedges = np.histogram2d(x, y, bins=[x_edges, y_edges])

      # Calculate means
      bin_means = np.zeros_like(hist)
      for i in range(x_bins):
        for j in range(y_bins):
          # Get indexes of points in bin
          mask = ((x >= xedges[i]) & (x < xedges[i + 1]) &
                  (y >= yedges[j]) & (y < yedges[j + 1]))
          bin_means[i, j] = data.loc[mask, 'score'].mean() if mask.any() else np.nan

      # Couldn't get hist2d to cooperate by setting `weights`, `pcolormesh` will do.
      hist = plt.pcolormesh(x_edges, y_edges, bin_means.T, cmap=cmap)
      return (hist, hist)

    else:
      hist = ax.hist2d(x, y, bins=64, norm=LogNorm())
      return (hist, hist[3])

# === Main ===
if __name__ == "__main__":
  args = docopt(__doc__)

  data = load_data(args)

  fig, ax = plt.subplots(tight_layout=True)
  fig.patch.set_facecolor('#D9D9D9')
  
  # Clip score values to ignore high outliers (known layouts are assigned bogus scores)
  score_threshold = np.percentile(data['score'], 99)
  data['score'] = np.clip(data['score'], None, score_threshold)

  hist, mappable = make_hist(args, ax)

  set_axes_labels(args, ax, 2)
  label_known_layouts(args, ax, 2)
  
  # Colorbar
  cbar = fig.colorbar(mappable=mappable, ax=ax) # type: ignore[arg-type,index]

  if args['-S']:
    cbar.ax.invert_yaxis()
    cbar.set_label("Mean score / cell\n(lower/brighter is better)")
  else:
    cbar.set_label("Generated layout density\n(per ~1/64²)")
  
  # Export
  plt.title(args['-t'] + "\n" + f'({len(data):,} samples)')
  plt.savefig(args['-o'], dpi=180)

  print("Created", args['-o'])
