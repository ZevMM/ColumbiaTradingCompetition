FROM python:3.11-slim
WORKDIR /app
COPY . .
RUN pip install --no-cache-dir websockets numpy
CMD ["python", "MarketMaker.py"]
