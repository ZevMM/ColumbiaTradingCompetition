import {networkInterfaces} from 'os'
import { readFileSync, writeFileSync } from 'fs';

function getLocalIp() {
  const interfaces = networkInterfaces();
  for (let interfaceName in interfaces) {
    for (let i = 0; i < interfaces[interfaceName].length; i++) {
      const iface = interfaces[interfaceName][i];
      if (iface.family === 'IPv4' && !iface.internal) {
        return iface.address;
      }
    }
  }
  return 'localhost';
}

const localIp = getLocalIp();

const configFilePath = './src/local-ip.js';

let fileContent = `const addr = \"ws://${localIp}:443/ws\"; export default addr`;

writeFileSync(configFilePath, fileContent, 'utf8');
