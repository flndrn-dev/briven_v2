# Summary

# Briven v2

This page links to technical content about the Briven v2 database engine.

Public Briven product docs will live at `briven.tech`; this folder documents the engine internals used to build the Briven platform.

- [Briven Developer Entry](./briven.md)
- [Briven Control API](./briven-control-api.md)
- [Sprint 3 Status](./sprint-3-status.md)

# Engine Docs

The documents below describe the Briven engine architecture.

# Architecture

[Introduction]()
- [Separation of Compute and Storage](./separation-compute-storage.md)

- [Compute]()
  - [Postgres changes](./core_changes.md)

- [Pageserver](./pageserver.md)
    - [Services](./pageserver-services.md)
    - [Thread management](./pageserver-thread-mgmt.md)
    - [WAL Redo](./pageserver-walredo.md)
    - [Page cache](./pageserver-pagecache.md)
    - [Storage](./pageserver-storage.md)
    - [Compaction](./pageserver-compaction.md)
    - [Processing a GetPage request](./pageserver-processing-getpage.md)
    - [Processing WAL](./pageserver-processing-wal.md)

- [WAL Service](walservice.md)
  - [Consensus protocol](safekeeper-protocol.md)

- [Source view](./sourcetree.md)
  - [docker.md](./docker.md) — Docker images and building pipeline.
  - [Error handling and logging](./error-handling.md)

- [Glossary](./glossary.md)

# Uncategorized

- [authentication.md](./authentication.md)
- [multitenancy.md](./multitenancy.md) — how multitenancy is organized in the pageserver and inherited local CLI.
- [settings.md](./settings.md)
#FIXME: move these under sourcetree.md
#- [postgres_ffi/README.md](/libs/postgres_ffi/README.md)
#- [test_runner/README.md](/test_runner/README.md)


# RFCs

Major changes are documented in RFCS:
- See [RFCs](./rfcs/README.md) for more information
