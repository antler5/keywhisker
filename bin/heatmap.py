#!/usr/bin/env -S python -m pipx run

# /// script
# requires-python = ">=3.13"
# dependencies = [
#   "docopt",
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
"""

# === Imports ===
import sys

from typing import Any

from docopt import docopt

import matplotlib
import matplotlib.pyplot as plt
import pandas as pd

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
    df[col] = pd.NA
  data = pd.concat([data, df], ignore_index=True)

  return data

def set_axes_labels(args: dict, ax: matplotlib.axes._axes.Axes, dim: int) -> None:
  "Set and style axis labels from args."
  ax.set_facecolor('black')
  for i in range(0,dim):
    dim_letter = ['x','y','z'][i]

    set_label = getattr(ax, 'set_' + dim_letter + 'label')
    set_label(maybe_upper(args['-x']))

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

    ax.scatter(*pos, label=layout, marker='o', s=25, color=color, edgecolor='#555') # type: ignore[arg-type,misc]
    ax.annotate(layout,
                (pos[0]+0.35, *pos[1:]), # type: ignore[arg-type]
                va='center', color='black', fontsize=8,
                bbox=dict(facecolor='#D9D9D9', edgecolor='#555', boxstyle='round'))

# === Main ===
if __name__ == "__main__":
  args = docopt(__doc__)

  data = load_data(args)

  fig, ax = plt.subplots(tight_layout=True)
  fig.patch.set_facecolor('#D9D9D9')
  
  set_axes_labels(args, ax, 2)
  
  hist = (ax.hexbin(data[args['-x']], data[args['-y']], gridsize=(50,30), bins='log', linewidths=0)
          if args['-H'] else
          ax.hist2d(data[args['-x']], data[args['-y']], bins=64, norm=LogNorm()))

  label_known_layouts(args, ax, 2)
  
  # Colorbar
  cbar = fig.colorbar(hist if args['-H'] else hist[3], ax=ax) # type: ignore[arg-type,index]
  cbar.set_label("Generated layout density\n(per ~1/64²)")
  
  # Export
  plt.title(args['-t'] + "\n" + f'({len(data):,} samples)')
  plt.savefig(args['-o'])

  print("Created", args['-o'])
