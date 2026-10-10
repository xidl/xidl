# npm packages

`xidlc` is published to npm as a small launcher plus one package per platform.
The launcher resolves the platform package at runtime, so installing the CLI
needs no install script and no network access beyond the registry.

| Package             | Contents                                              |
| ------------------- | ----------------------------------------------------- |
| `xidlc`             | `bin/xidlc.js` launcher and the `src/` platform table |
| `xidlc-<os>-<arch>` | the prebuilt binary under `bin/`                      |

`xidlc/src/platforms.js` is the single source of truth for the platform key, npm
package and Rust target of every supported platform. The tests in
`tests/packages.test.mjs` fail when the platform table, the package manifests,
`release-please-config.json` or the release workflow drift apart.

## Binaries are not committed

Binaries come from the GitHub release of the matching version and are ignored by
git (`/packages/xidlc-*/bin/`). Fetch them with:

```bash
just npm-fetch-binaries
```

The script downloads `xidlc-<rust-target>.tar.gz` for every platform, extracts
the binary, verifies its executable format, and writes it into the platform
package. Useful flags: `--version` for another release, `--base-url` to download
through a mirror, `--repo owner/name` for a fork, `--ref <name>` to assert that
a CI ref matches the package version.

## First publish (bootstrap)

npm only accepts a trusted publisher configuration for a package that already
exists, so the first publish of all 7 packages is manual:

```bash
just npm-fetch-binaries   # fill packages/xidlc-*/bin with the version's binaries
npm login
just npm-publish          # publishes the 6 platform packages, then xidlc
```

Then configure a trusted publisher for each of the 7 packages on npmjs.com:
repository `xidl/xidl`, workflow `publish-release.yml`, environment left empty.
The wrapper and the platform packages share the version, so every package must
be published together.

## Publishing from CI

`.github/workflows/publish-release.yml` publishes on every `v*` tag once the
release binaries are uploaded, authenticated with the npm id-token (no
`NPM_TOKEN` secret). The script publishes the platform packages first and skips
versions that are already on the registry, which makes re-runs safe. Add
`--dry-run` to see what would be published without uploading.

## Adding a platform

Add the platform to `xidlc/src/platforms.js`, create `<package>/package.json`
next to the other packages, add the Rust target to the release workflow matrix,
and add the version fields to `release-please-config.json`. `just test-npm`
reports every piece that is missing.
