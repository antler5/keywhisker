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
"""

# === Imports ===
import sys

from docopt import docopt

import pandas as pd
import matplotlib.pyplot as plt

from matplotlib.colors import LogNorm

# === Variables ===

# Persistant viewport
range = [[1.38, 13.27], [15.34, 55]]

# Known layouts
known_layouts = {
  # label: [m3roll,sfb,sfs]
  "ardux": [1.32,92.26,90.93],
  "ardux-no-spc": [2.92,54.56,58.45],
  "artsey": [1.38,90.90,90.19],
  "artsey-no-spc": [3.02,53.16,58.23],
  "caret": [5.72,52.58,50.16],
  "caret-no-spc": [9.92,34.25,50.20],
  "taipo": [9.69,31.11,33.85],
  "taipo-no-spc": [4.31,49.18,51.38],
}

# === Main ===
if __name__ == "__main__":
  args = docopt(__doc__)

  data = pd.read_csv(args['FILE'], sep='\t')
  
  fig, ax = plt.subplots(tight_layout=True)
  hist = ax.hist2d(data[args['-x']], data[args['-y']], bins=64, range=range, norm=LogNorm())
  
  ax.set_xlabel(args['-x'])
  ax.set_ylabel(args['-y'])
  ax.set_facecolor('black')
  
  # Known layouts
  for l,v in known_layouts.items():
    ax.scatter(v[0], v[1], label=l, marker='o', s=50, color='orange')
    ax.annotate(l, (v[0] + 0.25, v[1] - 0.425), color='white', fontsize=9)
  
  # Colorbar
  cbar = fig.colorbar(hist[3], ax=ax)
  cbar.set_label("Generated layout density\n(per 1/64-scale cell)")
  
  # Export
  plt.title(args['-t'])
  plt.savefig(args['-o'])

  print("Created", args['-o'])
