import matplotlib.pyplot as plt

# Read numbers from file
with open("./TS_demo", "r") as file:
    data = [float(line.strip()) for line in file]

# Generate time points
time_points = range(len(data))

# Plot time series
plt.figure(figsize=(10, 5))
plt.plot(time_points, data, marker='o', linestyle='-')
plt.xlabel("Time")
plt.ylabel("Value")
plt.title("Time Series Chart")
plt.grid(True)
plt.show()