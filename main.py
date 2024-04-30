import pandas as pd
import matplotlib.pyplot as plt

data = pd.read_csv('./data/data.csv')

for (name, vals) in data.items():
    print(name)
    print(f"  mean: {vals.mean()}")
    print(f"  std: {vals.std()}")

qwerty = [38.21, 47.99]
    
fig, ax = plt.subplots(tight_layout=True)
hist = ax.hist2d(data.roll, data["skipalt"], bins=(64, 64))
ax.set_xlabel('Rolls')
ax.set_ylabel('Skip-alternation')
ax.scatter(qwerty[0], qwerty[1], label='qwerty', marker='o', s=100, color='red')

plt.show()

