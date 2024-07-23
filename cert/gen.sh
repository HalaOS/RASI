openssl req -new -x509 -batch -nodes -days 10000 -keyout rasi_ca.key -out rasi_ca.pem

openssl req -new -batch -nodes -sha256 -keyout server.key -out server.csr -subj '/C=GB/CN=rasi.quic'
openssl x509 -req -days 10000 -in server.csr -CA rasi_ca.pem -CAkey rasi_ca.key -CAcreateserial -out server.crt
openssl verify -CAfile rasi_ca.pem server.crt

openssl req -new -batch -nodes -sha256 -keyout client.key -out client.csr -subj '/C=GB/CN=rasi.quic'
openssl x509 -req -days 10000 -in client.csr -CA rasi_ca.pem -CAkey rasi_ca.key -CAcreateserial -out client.crt
openssl verify -CAfile rasi_ca.pem client.crt



rm client.csr
rm server.csr
rm rasi_ca.key