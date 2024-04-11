import numpy as np
import pandas as pd
from matplotlib import pyplot as plt
from sklearn.linear_model import LinearRegression
from sklearn.model_selection import train_test_split

data = pd.read_csv('./data/data.csv')
X = data[['skiproll', 'skipalt', 'sfb']]
y = data['roll']
X_train, X_test, y_train, y_test = train_test_split(X, y, test_size=0.33, random_state=1)
regression = LinearRegression().fit(X_train, y_train)
print(regression.coef_)
print(regression.intercept_)

p_err = 0
a_err = 0
val = 0
num = 100000
for i, (skiproll, skipalt, sfb) in X_test[:num].iterrows():
    predicted = skiproll * regression.coef_[0] + skipalt * regression.coef_[1] + sfb * regression.coef_[2] + regression.intercept_
    val += y_test[i]
    p_err += abs(predicted - y_test[i]) / predicted
    a_err += predicted - y_test[i]
    print(p_err)

print(f"mean value: {val / num}")
print(f"mean error: {a_err / num:.2f}")
print(f"mean percentage error: {p_err * 100 / num:.2f}%")

# plt.scatter(X, y)
# plt.plot(X, np.dot(X, regression.coef_) + regression.intercept_, color='r')
# plt.xlabel('Roll')
# plt.ylabel('Skip Roll')
# plt.show()
