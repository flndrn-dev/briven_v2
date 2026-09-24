# Briven v2 GitHub Workflows

This directory contains Briven repository automation. Some inherited engine-maintenance
workflows still need replacement before they can run against Briven-owned infrastructure,
registries, storage, and release credentials.

For Sprint 2, Briven branding is applied to the public contributor path:

- Issue templates.
- Pull request templates.
- README/docs entry points.
- Docker image naming targets.

Workflow internals should be rebranded or replaced as Briven-owned infrastructure comes
online. Several jobs depend on existing secrets, image names, benchmark labels, and action
paths, so each workflow needs a focused migration with a testable replacement for:

- Container registry packages.
- CI secrets.
- Benchmark storage.
- Release process.
- Build-tool images.

Until then, remaining compatibility workflow names are treated as engine-maintenance internals,
not as Briven product identity.
