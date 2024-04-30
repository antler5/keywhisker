import pandas as pd
import matplotlib.pyplot as plt
import numpy as np

greedy_naive = pd.read_csv(f'data/greedy_runs.csv')
greedy_neighbor = pd.read_csv(f'data/greedy_neighbor_runs.csv')

fig, ax = plt.subplots(tight_layout=True)
hist = ax.hist(greedy_naive.amount, bins=32)
hist2 = ax.hist(greedy_neighbor.amount, bins=32)
plt.show()
