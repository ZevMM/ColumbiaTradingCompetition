import math
import pyaudio
import numpy as np
import matplotlib.pyplot as plt
import audioop
import threading
import atexit
import time

CHUNK = 256
FORMAT = pyaudio.paInt16
CHANNELS = 2
RATE = 20000
RECORD_SECONDS = 10
WAVE_OUTPUT_FILENAME = "output.wav"

lock = threading.Lock()

#gives average volume since last query
class AD:
    def __init__(self):
        self.p = pyaudio.PyAudio()
        self.stream = self.p \
            .open(format=FORMAT, channels=CHANNELS, rate=RATE, input=True, frames_per_buffer=CHUNK)
        self.data = []
        self.hist = []
        listener = threading.Thread(target=self.__start)
        listener.start()
        atexit.register(self.__end)

    def pull(self):
        with lock:
            toreturn = round(np.mean(self.data),2)
            self.data.clear()
            self.hist.append(toreturn)
            return ((math.atan((toreturn - np.mean(self.hist)) / 7) /  (math.pi)) + 0.5)*86 + 8

    def __start(self):
        while(True):
            data = self.stream.read(CHUNK)
            rms = (audioop.rms(data, 2))
            with lock:
                self.data.append(rms)
    
    def __end(self):
        self.stream.stop_stream()
        self.stream.close()
        self.p.terminate()


if __name__ == "__main__":
    generator = AD()
    f = open('AD_demo', 'a+')
    time.sleep(5)
    for i in range(65):
        f.write(str(generator.pull()) + "\n")
        f.flush()
        time.sleep(45)
    f.close()


#alldata = []
#y_vals = []
#x_vals = []
#count = 1

'''
for i in range(1, int(RATE / CHUNK * RECORD_SECONDS) + 1):
    data = stream.read(CHUNK)
    rms = (audioop.rms(data, 2))
    alldata.append(rms)
    if (i % 120 == 0):
        toAppend = round(np.mean(alldata),2)
        y_vals.append(toAppend)
        file.write(str(toAppend) + '\n')
        x_vals.append(count)
        count += 1
        alldata = []



file.close()

plt.plot(x_vals, y_vals, label='Data Points', marker='o')
plt.xlabel('Time')
plt.ylabel('Avg Volume')
plt.show()
'''

