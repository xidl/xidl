# npm packaging for xidlc

This repository now includes a prototype npm workspace under `npm/` for
publishing `xidlc` as a native CLI package.

## Package model

- `@cathaysia/xidlc`: main package that exposes the `xidlc` command
- `@cathaysia/xidlc-darwin-arm64`
- `@cathaysia/xidlc-darwin-x64`
- `@cathaysia/xidlc-linux-arm64`
- `@cathaysia/xidlc-linux-x64`
- `@cathaysia/xidlc-win32-arm64`
- `@cathaysia/xidlc-win32-x64`

The main package uses npm `optionalDependencies` and a small Node launcher to
resolve the installed platform package at runtime.

## Why this shape

The existing release workflow already produces signed or checksummed native
archives for each supported target. npm only needs to repack those release
artifacts into platform-scoped packages and publish one coordinating package.

This avoids postinstall downloads and keeps npm installs deterministic.

## Stage release assets

1. Build or download the normal release assets into a local directory.
2. Run:

```bash
cd npm
XIDLC_NPM_VERSION=0.47.0 \
XIDLC_RELEASE_DIR=/path/to/release-assets \
npm run stage:release
```

The script extracts each archive and copies the binary into the matching npm
platform package.

## Publish order

1. Publish all platform packages.
2. Publish `@cathaysia/xidlc` last.

That order matters because the main package references the platform packages as
`optionalDependencies` at the same version.

## Next step

The remaining work is CI automation: add a workflow that stages release assets,
packs each npm workspace, and publishes to npm after the GitHub Release assets
exist.
