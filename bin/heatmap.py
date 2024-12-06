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
  -o OUT    Output file. [default: img.png]
  -H        Use hexbins instead of hist2d.
"""

# === Imports ===
import sys

from docopt import docopt

import pandas as pd
import matplotlib.pyplot as plt

from matplotlib.ticker import PercentFormatter
from matplotlib.colors import LogNorm
from matplotlib.lines import Line2D

# === Variables ===

# Persistant viewport
# range = [[1.38, 13.27], [15.34, 55]]
# range = [[0, 55], [0, 55]]
range = None

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

known_layouts = {
  # "ardux":         row(1.32, 92.26, 90.93),
  "ardux-no-spc":  row(2.92, 54.56, 58.45),
  # "artsey":        row(1.38, 90.90, 90.19),
  "artsey-no-spc": row(3.02, 53.16, 58.23),
  "caret":         row(5.72, 52.58, 50.16),
  "caret-no-spc":  row(9.92, 34.25, 50.20),
  "taipo":         row(9.69, 31.11, 33.85),
  "taipo-no-spc":  row(4.31, 49.18, 51.38)
}

# === Helpers ===
def maybe_upper(s: str) -> bool:
  return s.upper() if s.isalpha() else s

def append_intersection(self: pd.DataFrame, row: dict) -> None:
  "Append ROW to SELF, keeping only columns which intersect."
  self.loc[-1] = pd.Series()
  for l, v in row.items():
    if l in self.loc[-1].keys():
      self.at[-1, l] = v
  self.index += 1

pd.core.frame.DataFrame.append_intersection = append_intersection # type: ignore[attr-defined]
del append_intersection

# === Main ===
if __name__ == "__main__":
  args = docopt(__doc__)

  data = pd.read_csv(args['FILE'], sep='\t')
  fig, ax = plt.subplots(tight_layout=True)
  fig.patch.set_facecolor('#D9D9D9')
  
  ax.set_xlabel(maybe_upper(args['-x']))
  ax.xaxis.set_major_formatter(PercentFormatter(decimals=0))

  ax.set_ylabel(maybe_upper(args['-y']))
  ax.yaxis.set_major_formatter(PercentFormatter(decimals=0))

  ax.set_facecolor('black')
  
  # Known layouts
  best = data.loc[data['score'].idxmin()]
  known_layouts['best'] = best
  for metrics in known_layouts.values():
    data.append_intersection(metrics)

  if args['-H']:
    hist = ax.hexbin(data[args['-x']], data[args['-y']], gridsize=[50,30], bins='log', linewidths=(0,))
  else:
    hist = ax.hist2d(data[args['-x']], data[args['-y']], bins=64, range=range, norm=LogNorm())

  for layout, metrics in known_layouts.items():
    x = metrics[args['-x']]
    y = metrics[args['-y']]
    color = 'red' if layout == 'best' else 'orange'
    ax.scatter(x, y, label=layout, marker='o', s=25, color=color, edgecolor='#555')
    ax.annotate(layout, (x+0.35, y), va='center', color='black', fontsize=8, bbox=dict(facecolor='#D9D9D9', edgecolor='#555', boxstyle='round'))
  
  # Colorbar
  if args['-H']:
    cbar = fig.colorbar(hist, ax=ax)
  else:
    cbar = fig.colorbar(hist[3], ax=ax)
  cbar.set_label("Generated layout density\n(per ~1/64²)")
  
  # Export
  plt.title(args['-t'] + "\n" + f'({len(data):,} samples)')
  plt.savefig(args['-o'],
              dpi=180 if args['-H'] else None) # .svg recommended

  print("Created", args['-o'])
