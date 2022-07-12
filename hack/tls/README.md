FROM: https://jaanus.com/ios-13-certificates/

update `IP.1` in `config.cnf` with your dev machine's IP

```
openssl genrsa -out development-ca.key 4096
openssl req -x509 -new -nodes -key development-ca.key -sha256 -days 365 -out development-ca.crt
openssl genrsa -out development.key 4096
openssl req -new -key development.key -config config.cnf -out development.csr
openssl x509 -req -in development.csr -CA development-ca.crt -CAkey development-ca.key -CAcreateserial -out development.crt -days 365 -sha256 -extfile config.cnf -extensions req_ext
```

on windows if these commands fail in the container they can be run can run on the host system

find instructions for adding `development-ca.crt` as a trusted root certificate on your test devices -

windows/chrome:
https://docs.microsoft.com/en-us/skype-sdk/sdn/articles/installing-the-trusted-root-certificate

firefox:
https://docs.vmware.com/en/VMware-Adapter-for-SAP-Landscape-Management/2.1.0/Installation-and-Administration-Guide-for-VLA-Administrators/GUID-0CED691F-79D3-43A4-B90D-CD97650C13A0.html

ios:
https://www.theictguy.co.uk/adding-trusted-root-certificates-to-ios14/
