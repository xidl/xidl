const fs = require('node:fs');
const path = require('node:path');

const PLATFORM_PACKAGES = {
  darwin: {
    arm64: '@cathaysia/xidlc-darwin-arm64',
    x64: '@cathaysia/xidlc-darwin-x64',
  },
  linux: {
    arm64: '@cathaysia/xidlc-linux-arm64',
    x64: '@cathaysia/xidlc-linux-x64',
  },
  win32: {
    arm64: '@cathaysia/xidlc-win32-arm64',
    x64: '@cathaysia/xidlc-win32-x64',
  },
};

function resolveBinaryPath() {
  const packageName = PLATFORM_PACKAGES[process.platform]?.[process.arch];
  if (!packageName) {
    throw new Error(
      `Unsupported platform for @cathaysia/xidlc: ${process.platform} ${process.arch}`,
    );
  }

  let packageJsonPath;
  try {
    packageJsonPath = require.resolve(`${packageName}/package.json`);
  } catch {
    throw new Error(
      `The platform package ${packageName} is not installed. Reinstall @cathaysia/xidlc on ${process.platform} ${process.arch}.`,
    );
  }

  const packageDir = path.dirname(packageJsonPath);
  const binaryName = process.platform === 'win32' ? 'xidlc.exe' : 'xidlc';
  const binaryPath = path.join(packageDir, 'bin', binaryName);
  if (!fs.existsSync(binaryPath)) {
    throw new Error(`Missing xidlc binary in ${packageName}`);
  }

  return binaryPath;
}

module.exports = {
  resolveBinaryPath,
};
