import WebSocket from 'ws';
import { readFileSync } from 'fs';
import { Agent } from 'https';

const cert = readFileSync('src/nginx.crt');

const ws = new WebSocket("wss://192.168.1.73:443/ws", ["zem2109"])


ws.onmessage = (e) => console.log(e)