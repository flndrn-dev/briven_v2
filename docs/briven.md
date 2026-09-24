# Briven Developer Entry

Briven v2 is the clean database engine foundation for `briven.tech`.

This repository presents the public project as Briven. Normal Briven work uses the
Briven repository as the source of truth:

```text
origin   https://github.com/flndrn-dev/briven_v2.git
```

## What To Rename

Safe early branding work:

- Public README and docs entry points.
- Briven logo, icon, and favicon.
- Product language in website/dashboard surfaces.
- Docker image tags and deployment labels once build/test coverage exists for the change.
- Briven-facing local command wrappers where they do not break inherited engine tooling.

Do not blindly rename:

- Rust crate names that are referenced by many internal build targets.
- Internal compatibility binary names.
- Internal module names.
- Test fixture names.
- Protocol names.
- Paths that inherited engine tooling expects.

Those names should be changed behind Briven-facing aliases first, then migrated with focused
tests when the engine layer is stable.

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
./target/debug/briven_local init
./target/debug/briven_local start
./target/debug/briven_local tenant create --set-default
./target/debug/briven_local endpoint create main
./target/debug/briven_local endpoint start main
```

Stop locally:

```bash
./target/debug/briven_local stop
```
