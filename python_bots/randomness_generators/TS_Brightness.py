from selenium import webdriver
from selenium.webdriver.common.by import By
import time
from PIL import Image
import matplotlib.pyplot as plt
import math
import random
#import statistics

def calculate_average_pixel_value(image_path):
    image = Image.open(image_path)
    image = image.convert('L')
    pixel_data = list(image.getdata())
    average_pixel_value = sum(pixel_data) / len(pixel_data)
    return average_pixel_value

class TS:
    def __init__(self):
        self.hist = []
        driver = webdriver.Edge()
        driver.get("https://www.earthcam.com/usa/newyork/timessquare/?cam=tsrobo1")
        self.livestream = driver.find_element(By.ID,value= 'videoPlayer_html5_api')

    def pull(self):
        self.livestream.screenshot("timessquare.png")
        avg = calculate_average_pixel_value("timessquare.png")
        self.hist.append(avg)
        runavg = sum(self.hist) / len(self.hist)
        return ((math.atan((avg - runavg) / 18) /  (math.pi)) + 0.5)*50





'''
file = open('TS_data', 'a+')



#y_vals = []
#x_vals = []

data = []
driver = webdriver.Edge()
driver.get("https://www.earthcam.com/usa/newyork/timessquare/?cam=tsrobo1")
livestream = driver.find_element(By.ID,value= 'videoPlayer_html5_api')

time.sleep(30)

#livestream.screenshot("timessquare.png")
#toAppend = calculate_average_pixel_value("timessquare.png")
#data.append(toAppend)

for i in range(60):
    livestream.screenshot("timessquare.png")
    toAppend = calculate_average_pixel_value("timessquare.png")

    data.append(toAppend)
    runavg = sum(data) / len(data)
    #print(statistics.stdev(data))
    #y_vals.append(toAppend)
    #x_vals.append(i)
    # I feel like the 20 should be dynamic in some way, like maybe one standard deviation
    file.write(str(((math.atan((toAppend - runavg) / 18) /  (math.pi)) + 0.5)*50) + '\n')
    offset = random.randint(0,5)
    time.sleep(60 + offset)

file.close()
#plt.plot(x_vals, y_vals, label='Data Points', marker='o')
#plt.xlabel('Time')
#plt.ylabel('Avg Brightness')
#plt.show()
'''