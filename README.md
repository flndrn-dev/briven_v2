<p align="center">
  <img src="./branding/logo.svg" width="96" alt="Briven logo" />
</p>

# Briven

Briven is a serverless Postgres platform foundation for `briven.tech`.

This repository is the clean Briven v2 database engine baseline. It is derived from the open-source Neon architecture: stateless Postgres compute, separate storage, pageserver, safekeepers, WAL durability, timelines, and copy-on-write branches.

The goal is to build Briven as a revenue-ready Postgres + pgvector + AI platform without carrying over the old Doltgres/Supabase-style platform code from Briven v1.

## Current Status

Sprint 1 is complete:

- Clean Neon-derived repository created.
- Briven public origin configured at `https://github.com/flndrn-dev/briven_v2.git`.
- Neon kept as maintainer upstream at `https://github.com/neondatabase/neon.git`.
- Local macOS build verified.
- Briven logo, icon, and favicon copied into [`branding/`](./branding/).

Sprint 2 is in progress:

- Public-facing Briven branding.
- Safe README/docs entry points.
- No fragile internal engine renames yet.

## Architecture

Briven follows the Neon-style separation of compute and storage:

- **Compute**: stateless Postgres nodes.
- **Pageserver**: scalable page storage backend for compute.
- **Safekeepers**: redundant WAL service for durable write-ahead log storage.
- **Object storage**: long-term storage for history and layers.
- **Timelines/branches**: copy-on-write database history.

See the developer docs in [`docs/SUMMARY.md`](./docs/SUMMARY.md), especially:

- [`docs/separation-compute-storage.md`](./docs/separation-compute-storage.md)
- [`docs/pageserver.md`](./docs/pageserver.md)
- [`docs/walservice.md`](./docs/walservice.md)

## Repository Remotes

Normal Briven work should use:

```bash
git clone --recursive https://github.com/flndrn-dev/briven_v2.git
cd briven_v2
```

Maintainers should keep Neon upstream available for security fixes and engine improvements:

```bash
git remote -v
# origin   https://github.com/flndrn-dev/briven_v2.git
# upstream https://github.com/neondatabase/neon.git
```

## Build Locally

The path to the source tree must not contain spaces.

### macOS

Install dependencies:

```bash
xcode-select --install
brew install protobuf openssl flex bison icu4c pkg-config m4 libpq
brew link --force libpq
```

If Apple compiler tools are blocked, accept the Xcode license:

```bash
sudo xcodebuild -license accept
```

Install Rust:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup toolchain install 1.88.0 --profile default --component llvm-tools --component rustfmt --component clippy
```

Build:

```bash
make -j$(sysctl -n hw.logicalcpu) -s
```

### Linux

On Ubuntu/Debian:

```bash
apt install build-essential libtool libreadline-dev zlib1g-dev flex bison libseccomp-dev \
  libssl-dev clang pkg-config libpq-dev cmake postgresql-client protobuf-compiler \
  libprotobuf-dev libcurl4-openssl-dev openssl python3-poetry lsof libicu-dev
```

Build:

```bash
make -j$(nproc) -s
```

## Run A Local Briven Database

After a successful build:

```bash
./target/debug/neon_local init
./target/debug/neon_local start
./target/debug/neon_local tenant create --set-default
./target/debug/neon_local endpoint create main
./target/debug/neon_local endpoint start main
```

The local Postgres endpoint is shown by the final command. It is typically:

```text
postgresql://cloud_admin@127.0.0.1:55432/postgres
```

Stop the local stack:

```bash
./target/debug/neon_local stop
```

## Branding

Briven brand assets live in [`branding/`](./branding/):

- [`branding/logo.svg`](./branding/logo.svg)
- [`branding/icon.svg`](./branding/icon.svg)
- [`branding/favicon.svg`](./branding/favicon.svg)

The current Briven accent is `#00e87a`.

## Upstream And License

This codebase is derived from Neon’s open-source repository and remains Apache-2.0 licensed.

Required license and attribution notices are preserved in [`LICENSE`](./LICENSE) and [`NOTICE`](./NOTICE). Briven branding does not imply endorsement by Neon.
