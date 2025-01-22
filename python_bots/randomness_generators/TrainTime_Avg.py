from selenium import webdriver
from selenium.webdriver.common.by import By
import time
from PIL import Image
import matplotlib.pyplot as plt
import math
import random

class TT:
    def __init__(self):
        self.driver = webdriver.Edge()
        self.driver.get("https://realtimerail.nyc/stops/117")
        self.hist = []

    def pull(self):
        myl = []
        times = self.driver.find_elements \
            (By.CLASS_NAME, value= "time")
        for n in range(len(times)):
            myl.append(int("0" + "".join \
                ([s for s in times[n].text if s.isdigit()])))
        avg = sum(myl) / len(myl)
        self.hist.append(avg)
        runavg = sum(self.hist) / len(self.hist)
        self.driver.refresh()
        return (((math.atan((avg - runavg) / 6) /  (math.pi)) + 0.5)*50) 





'''
#doesn't account for time = "now", but that ends up adding more variance
file = open('Train_data','a+')
ifile = open('iTrain_data','a+')
driver = webdriver.Edge()
driver.get("https://realtimerail.nyc/stops/117")
time.sleep(10)
y_vals = []
#x_vals = []
#if we actually want avg wait time, should use the difference between each time
tot = 0
count = 0

for i in range(60):
    myl = []
    for n in range(5):
        times = driver.find_elements(By.CLASS_NAME, value= "time")
        if (n < len(times)):
            myl.append(int("0" + "".join([s for s in times[n].text if s.isdigit()])))
        else:
            myl.append(0)
    avg = sum(myl)   
    ##avg = sum(int("0" + "".join([s for s in t.text if s.isdigit()])) for t in times)
    count += 1
    tot += avg
    #y_vals.append(avg)
    #x_vals.append(i)
    runavg = tot / count
    file.write(str(((math.atan((avg - runavg) / 6) /  (math.pi)) + 0.5)*50) + '\n')
    ifile.write(str(50 - ((math.atan((avg - runavg) / 6) /  (math.pi)) + 0.5)*50) + '\n')
    driver.refresh()
    offset = random.randint(0,5)
    time.sleep(60 + offset)

file.close()
ifile.close()
#plt.plot(x_vals, y_vals, label='Data Points', marker='o')
#plt.xlabel('Time')
#plt.ylabel('Avg Arrival')
#plt.show()
'''