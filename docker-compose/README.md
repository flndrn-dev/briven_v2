# Example docker compose configuration

The configuration in this directory is used for testing Briven engine Docker images. It is
not intended for deploying a usable system. To run a development environment where
you can experiment with a miniature Briven system, use `./target/debug/briven_local`
rather than container images.

This configuration does not start the storage controller, because the controller
needs a way to reconfigure running computes, and no such thing exists in this setup.

The default image names are Briven-facing:

```text
BRIVEN_ENGINE_IMAGE=ghcr.io/flndrn-dev/briven-engine
BRIVEN_COMPUTE_REPOSITORY=ghcr.io/flndrn-dev
BRIVEN_COMPUTE_IMAGE=briven-compute-node-v${PG_VERSION}
BRIVEN_TEST_EXTENSIONS_IMAGE=ghcr.io/flndrn-dev/briven-test-extensions-v${PG_VERSION}
```

Compatibility image names can still be supplied by maintainers when comparing engine changes,
but Briven image names are the default path.

## Generating the JWKS for a compute

```shell
openssl genpkey -algorithm Ed25519 -out private-key.pem
openssl pkey -in private-key.pem -pubout -out public-key.pem
openssl pkey -pubin -inform pem -in public-key.pem -pubout -outform der -out public-key.der
key="$(xxd -plain -cols 32 -s -32 public-key.der)"
key_id="$(printf '%s' "$key" | sha256sum | awk '{ print $1 }' | basenc --base64url --wrap=0)"
x="$(printf '%s' "$key" | basenc --base64url --wrap=0)"
```
