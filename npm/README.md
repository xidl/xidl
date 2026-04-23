# xidlc npm prototype

This directory contains an npm distribution prototype for `xidlc`.

## Layout

- `packages/xidlc`: main CLI package published to npm
- `packages/xidlc-*`: platform packages that carry the native binary
- `scripts/stage-release-assets.mjs`: expands GitHub release archives into the
  platform package `bin/` directories before publishing

## Local checks

```bash
cd npm
npm run check
```

## Stage binaries from a release

```bash
cd npm
XIDLC_NPM_VERSION=0.47.0 \
XIDLC_RELEASE_DIR=/path/to/release-assets \
npm run stage:release
```

`XIDLC_RELEASE_DIR` should contain the archives produced by the existing release
workflow, for example `xidlc-aarch64-apple-darwin.tar.gz`.
