#!/bin/sh
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only

# mkcert.sh IP: makes the lab's CA, name-constrained to IP alone so its key
# can vouch for no other host, and the lab server's certificate for IP, in
# tls/. An existing CA is kept, so devices that trust it keep working; only
# the server certificate is made again.
set -eu
ip=${1:?usage: mkcert.sh IP}
dir=$(dirname "$0")/tls
mkdir -p "$dir"
if [ ! -f "$dir/ca.key" ]; then
  openssl req -x509 -new -newkey rsa:2048 -nodes -days 365 \
    -keyout "$dir/ca.key" -out "$dir/ca.crt" \
    -subj "/O=Strims dev lab/CN=Strims dev lab CA $ip" \
    -addext "basicConstraints=critical,CA:TRUE" \
    -addext "keyUsage=critical,keyCertSign,cRLSign" \
    -addext "nameConstraints=critical,permitted;IP:$ip/255.255.255.255"
  chmod 600 "$dir/ca.key"
fi
cat >"$dir/ext.cnf" <<CNF
basicConstraints=CA:FALSE
keyUsage=critical,digitalSignature,keyEncipherment
extendedKeyUsage=serverAuth
subjectAltName=IP:$ip
CNF
openssl req -new -newkey rsa:2048 -nodes -keyout "$dir/server.key" \
  -out "$dir/server.csr" -subj "/O=Strims dev lab/CN=$ip"
chmod 600 "$dir/server.key"
openssl x509 -req -in "$dir/server.csr" -CA "$dir/ca.crt" -CAkey "$dir/ca.key" \
  -CAcreateserial -days 365 -out "$dir/server.crt" -extfile "$dir/ext.cnf"
