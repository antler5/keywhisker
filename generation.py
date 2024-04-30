import pandas as pd
import matplotlib.pyplot as plt

finals = []
times = []
for i in range(0, 500000):
    data = pd.read_csv(f'./data/{i}.csv')
    finals.append(data.amount.iloc[-1])
    times.append(data.iteration.iloc[-1])
    # plt.plot(datas[i].iteration, datas[i].amount)

fig, ax = plt.subplots(tight_layout=True)
hist = ax.hist2d(times, finals, bins=40)
plt.show()
