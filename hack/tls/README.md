FROM: https://jaanus.com/ios-13-certificates/

```
openssl genrsa -out development-ca.key 4096
openssl req -x509 -new -nodes -key development-ca.key -sha256 -days 365 -out development-ca.crt
openssl genrsa -out development.key 4096
openssl req -new -key development.key -config config.cnf -out development.csr
openssl req -new -key development.key -config config.cnf -out development.csr
openssl x509 -req -in development.csr -CA development-ca.crt -CAkey development-ca.key -CAcreateserial -out development.crt -days 365 -sha256 -extfile config.cnf -extensions req_ext
```
