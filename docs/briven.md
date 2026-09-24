# Briven Developer Entry

Briven v2 is the clean database engine foundation for `briven.tech`.

This repository is derived from the open-source Neon architecture. Briven keeps Neon available as a maintainer upstream while presenting the public project as Briven:

```text
origin   https://github.com/flndrn-dev/briven_v2.git
upstream https://github.com/neondatabase/neon.git
```

## What To Rename

Safe early branding work:

- Public README and docs entry points.
- Briven logo, icon, and favicon.
- Product language in website/dashboard surfaces.
- Docker image tags and deployment labels once build/test coverage exists for the change.

Do not blindly rename:

- Rust crate names.
- Binary names.
- Internal module names.
- Test fixture names.
- Protocol names.
- Paths that upstream Neon tooling expects.

Those names can be changed later only with focused tests and a clear reason.

## Product Target

Briven is being built as:

- Serverless Postgres.
- pgvector-ready database infrastructure.
- AI application database platform.
- Stripe-backed paid SaaS.

The old Briven v1 Doltgres/Supabase-style platform code is not part of this repository.

## Current Local Commands

Build:

```bash
make -j$(sysctl -n hw.logicalcpu) -s
```

Run locally:

```bash
./target/debug/neon_local init
./target/debug/neon_local start
./target/debug/neon_local tenant create --set-default
./target/debug/neon_local endpoint create main
./target/debug/neon_local endpoint start main
```

Stop locally:

```bash
./target/debug/neon_local stop
```
