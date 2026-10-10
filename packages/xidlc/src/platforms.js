/**
 * Release and npm coordinates for every platform `xidlc` ships a binary for.
 *
 * Keys are `${process.platform}-${process.arch}`. `binary` is the executable
 * inside the platform package, `package` is the npm package that carries it,
 * and `target` is the Rust target triple of the GitHub release asset.
 */
export const PLATFORMS = {
  'darwin-arm64': {
    binary: 'xidlc',
    package: 'xidlc-darwin-arm64',
    target: 'aarch64-apple-darwin',
  },
  'darwin-x64': {
    binary: 'xidlc',
    package: 'xidlc-darwin-x64',
    target: 'x86_64-apple-darwin',
  },
  'linux-arm64': {
    binary: 'xidlc',
    package: 'xidlc-linux-arm64',
    target: 'aarch64-unknown-linux-musl',
  },
  'linux-x64': {
    binary: 'xidlc',
    package: 'xidlc-linux-x64',
    target: 'x86_64-unknown-linux-musl',
  },
  'win32-arm64': {
    binary: 'xidlc.exe',
    package: 'xidlc-win32-arm64',
    target: 'aarch64-pc-windows-msvc',
  },
  'win32-x64': {
    binary: 'xidlc.exe',
    package: 'xidlc-win32-x64',
    target: 'x86_64-pc-windows-msvc',
  },
};
