import numpy as np
import pandas as pd
from matplotlib import pyplot as plt
from sklearn.linear_model import LinearRegression
from sklearn.model_selection import train_test_split

data = pd.read_csv('./data/data.csv')
data.sort_values('roll')
X = data[['skipalt', 'sfb', 'sft']].to_numpy()
y = data['roll'].to_numpy()
X_train, X_test, y_train, y_test = train_test_split(X, y, test_size=0.5, random_state=1)
regression = LinearRegression().fit(X_train, y_train)
print(regression.coef_)
print(regression.intercept_)
print("train score", regression.score(X_train, y_train))
print("test score", regression.score(X_test, y_test))

predicted = regression.predict(X_test)
plt.plot(y_test, 'b+')
plt.plot(predicted, 'r+')
plt.show()
