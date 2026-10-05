#!/bin/sh
set -eu
umask 077
mkdir -p /ca-private /ca-public /proxy-tls
if [ ! -s /ca-private/ca.key ]; then
  openssl req -x509 -newkey rsa:3072 -nodes -sha256 -days 3650 \
    -subj '/CN=Briven private customer database CA' \
    -addext 'basicConstraints=critical,CA:TRUE' \
    -addext 'keyUsage=critical,keyCertSign,cRLSign' \
    -keyout /ca-private/ca.key -out /ca-private/ca.crt >/dev/null 2>&1
fi
openssl x509 -in /ca-private/ca.crt -checkend 2592000 -noout >/dev/null
cp /ca-private/ca.crt /ca-public/ca.crt
chmod 644 /ca-public/ca.crt
if [ ! -s /proxy-tls/server.crt ] || ! openssl x509 -in /proxy-tls/server.crt -checkend 2592000 -noout >/dev/null; then
  openssl req -new -newkey rsa:3072 -nodes -sha256 \
    -subj '/CN=briven-customer-db' -keyout /proxy-tls/server.key -out /proxy-tls/server.csr >/dev/null 2>&1
  printf 'subjectAltName=DNS:briven-customer-db\nbasicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\n' > /proxy-tls/extensions.cnf
  openssl x509 -req -in /proxy-tls/server.csr -CA /ca-private/ca.crt -CAkey /ca-private/ca.key \
    -CAcreateserial -days 365 -sha256 -extfile /proxy-tls/extensions.cnf -out /proxy-tls/server.crt >/dev/null 2>&1
fi
openssl verify -CAfile /ca-public/ca.crt -verify_hostname briven-customer-db /proxy-tls/server.crt >/dev/null
chown -R 1000:1000 /proxy-tls
chmod 755 /proxy-tls
chmod 600 /proxy-tls/server.key
chmod 644 /proxy-tls/server.crt
echo 'private customer proxy certificate ready'
