#!/bin/sh
set -eu
umask 077
mkdir -p /ca-private /ca-public /control-tls /auth-tls
if [ ! -s /ca-private/ca.key ]; then
  openssl req -x509 -newkey rsa:3072 -nodes -sha256 -days 3650 \
    -subj '/CN=Briven private database CA' \
    -addext 'basicConstraints=critical,CA:TRUE' \
    -addext 'keyUsage=critical,keyCertSign,cRLSign' \
    -keyout /ca-private/ca.key -out /ca-private/ca.crt >/dev/null 2>&1
fi
openssl x509 -in /ca-private/ca.crt -checkend 2592000 -noout >/dev/null
cp /ca-private/ca.crt /ca-public/ca.crt
chmod 644 /ca-public/ca.crt
for database in control auth; do
  folder="/${database}-tls"
  hostname="briven-${database}-db"
  if [ ! -s "$folder/server.crt" ] || ! openssl x509 -in "$folder/server.crt" -checkend 2592000 -noout >/dev/null; then
    openssl req -new -newkey rsa:3072 -nodes -sha256 \
      -subj "/CN=$hostname" -keyout "$folder/server.key" -out "$folder/server.csr" >/dev/null 2>&1
    printf 'subjectAltName=DNS:%s\nbasicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\n' "$hostname" > "$folder/extensions.cnf"
    openssl x509 -req -in "$folder/server.csr" -CA /ca-private/ca.crt -CAkey /ca-private/ca.key \
      -CAcreateserial -days 365 -sha256 -extfile "$folder/extensions.cnf" -out "$folder/server.crt" >/dev/null 2>&1
  fi
  openssl verify -CAfile /ca-public/ca.crt -verify_hostname "$hostname" "$folder/server.crt" >/dev/null
  chown -R 999:999 "$folder"
  chmod 755 "$folder"
  chmod 600 "$folder/server.key"
  chmod 644 "$folder/server.crt"
done
echo 'private database certificates ready'
