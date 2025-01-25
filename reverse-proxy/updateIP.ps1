$localIP = (Test-Connection -ComputerName (hostname) -Count 1 | 
  Select IPV4Address).IPV4Address

$servconf = "server.conf"
$newservconf = "temp_server.conf"
$nginxconf = "nginx.conf"
$newnginxconf = "nginx-1.27.3\conf\nginx.conf"

(Get-Content $servconf) | ForEach-Object {$_ -replace '{{LOCAL_IP}}', $localIP} | Set-Content $newservconf

(Get-Content $nginxconf) | ForEach-Object {$_ -replace '{{LOCAL_IP}}', $localIP} | Set-Content $newnginxconf

$env:OPENSSL_CONF = "C:\Program Files\OpenSSL-Win64\bin\openssl.cfg"

& "C:\Program Files\OpenSSL-Win64\bin\openssl.exe" req -new -key server.key -out server.csr -config temp_server.conf

& "C:\Program Files\OpenSSL-Win64\bin\openssl.exe" x509 -req -in server.csr -CA ca.crt -CAkey ca.key -out server.crt -days 365 -extensions v3_req -extfile temp_server.conf

& ".\\nginx-1.27.3\nginx.exe"