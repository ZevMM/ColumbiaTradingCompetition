import numpy as np
import matplotlib.pyplot as plt

# Parameters
num_points = 63
min_val, max_val = 40, 65

def generate_declining_random_data(num_points, min_val, max_val):
    x = np.linspace(0, 1, num_points)  # Normalized time steps
    trend = np.linspace(min_val, max_val, num_points)  # Linear decline
    noise = np.random.uniform(-10, 10, num_points) * x  # Declining randomness
    data = np.clip(trend + noise, min_val, max_val)  # Apply bounds
    return data

# Generate data
data = generate_declining_random_data(num_points, min_val, max_val)

# Plot the data
plt.plot(data, marker='o', linestyle='-', label='Declining Random Data')
plt.xlabel('Index')
plt.ylabel('Value')
plt.title('Random Data with Declining Variability')
plt.legend()
plt.show()

# Print data points
print(data.tolist())