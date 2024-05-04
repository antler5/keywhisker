import pandas as pd
import seaborn as sns
import matplotlib.pyplot as plt
import numpy as np

files = [('data/greedy_naive_runs.csv', '100k Naive greedy runs (5k iterations each)', (0, 0)),
         ('data/greedy_neighbor_runs.csv', '100k Deterministic greedy runs', (0, 1)),
         ('data/simulated_annealing_5k_runs.csv', '100k Simulated annealing runs (5k iterations each)', (1, 0)),
         ('data/simulated_annealing_20k_runs.csv', '100k Simulated annealing runs (20k iterations each)', (1, 1)),
         ('data/simulated_annealing_100k_runs.csv', '10k Simulated annealing runs (100k iterations each)', (2, 0)),
         ('data/simulated_annealing_1m_runs.csv', '10k Simulated annealing runs (1m iterations each)', (2, 1))]

files = [('data/roll_greedy_deterministic_runs.tsv', 'Optimizing rolls with 10k deterministic greedy runs', 0),
         ('data/inroll_greedy_deterministic_runs.tsv', 'Optimizing inrolls with 10k deterministic greedy runs', 1),
         ('data/e200_inroll_greedy_deterministic_runs.tsv', 'Optimizing inrolls with 1k deterministic greedy runs', 2)]

sns.set()
fig, axs = plt.subplots(3, tight_layout=True)
fig.suptitle('Resulting Roll values over massed generation runs with various algorithms')
for file, title, index in files:
    ax = axs[index]
    data = pd.read_csv(file, sep='\t')[:10000]
    minimum = data.amount.min()
    mincount = data.amount.value_counts()[minimum]
    maximum = data.amount.max()
    maxcount = data.amount.value_counts()[maximum]
    total = len(data)
    print(title)
    print(f'  min {minimum} occurred {mincount} times ({100*mincount/total}%)')
    print(f'  max {maximum} occurred {maxcount} times ({100*maxcount/total}%)')
    print(f'  mean {data.amount.mean()}')
    print(f'  median {data.amount.median()}')
    print(f'  std {data.amount.std()}')
    filtered = data[(data.amount > 20)]
    ax.hist2d(filtered.iteration, filtered.amount, bins=data.iteration.max() - data.iteration.min())
    # ax.set_xscale('log')
    # ax.set_xticks([0.5, 0.6, 1.0, 2.0])
    ax.set_title(title)
    ax.set(xlabel='# of swaps made', ylabel='Resulting metric percent')
    
plt.show()
