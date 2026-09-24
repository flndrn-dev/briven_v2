# Briven v2 GitHub Workflows

This repository is derived from Neon. Many workflow files and composite actions are still inherited from upstream and may refer to Neon-owned infrastructure, image registries, S3 buckets, or cloud APIs.

For Sprint 2, Briven branding is applied to public repository entry points only:

- Issue templates.
- Pull request templates.
- README/docs entry points.
- Docker image naming targets.

Do not blindly rename workflow internals yet. Several jobs depend on upstream Neon secrets, image names, benchmark labels, and action paths. Rebrand or replace those workflows only when Briven has its own:

- Container registry packages.
- CI secrets.
- Benchmark storage.
- Release process.
- Build-tool images.

Until then, inherited workflow names are treated as engine-maintenance internals.
