# Current Briven repositories

Verified on 5 October 2026 against local Git remotes, the September 30 build plans, and both Briven Dokploy Compose sources.

| Repository | Purpose | Usage |
| --- | --- | --- |
| [flndrn-dev/briven-website](https://github.com/flndrn-dev/briven-website) | Website, dashboard, API, Auth SDK, CLI and Studio | Live Dokploy service uses `main`; database migration work is on `sprint3-serverless-postgres`. |
| [flndrn-dev/briven_v2](https://github.com/flndrn-dev/briven_v2) | PostgreSQL storage engine and engine control API | This workspace; active engine work is on `sprint3-serverless-postgres`. |

`briven-platform` is the previous Next.js/Prisma platform. `briven-plugin` and `homebrew-briven` distribute that older platform's plugin and CLI. They are not dependencies of the current build. The current CLI source is `packages/cli` (`@briven/cli`) inside `briven-website`.

Git history mirrors of all three retirement candidates are preserved locally in `repository-backups/2026-10-05/`, excluded from commits. All three legacy repositories were deleted from GitHub on 5 October 2026 after deletion permission was authorized. A fresh account inventory confirmed that only the two current Briven repositories above remain.
