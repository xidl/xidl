import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import process from 'node:process';

const rootDir = path.resolve(import.meta.dirname, '..');
const packagesDir = path.join(rootDir, 'packages');
const releaseDir = process.env.XIDLC_RELEASE_DIR;
const version = process.env.XIDLC_NPM_VERSION;

if (!releaseDir) {
  throw new Error('XIDLC_RELEASE_DIR is required');
}

if (!version) {
  throw new Error('XIDLC_NPM_VERSION is required');
}

const targets = [
  {
    workspace: 'xidlc-darwin-arm64',
    archive: 'xidlc-aarch64-apple-darwin.tar.gz',
    binaryName: 'xidlc',
  },
  {
    workspace: 'xidlc-darwin-x64',
    archive: 'xidlc-x86_64-apple-darwin.tar.gz',
    binaryName: 'xidlc',
  },
  {
    workspace: 'xidlc-linux-arm64',
    archive: 'xidlc-aarch64-unknown-linux-musl.tar.gz',
    binaryName: 'xidlc',
  },
  {
    workspace: 'xidlc-linux-x64',
    archive: 'xidlc-x86_64-unknown-linux-musl.tar.gz',
    binaryName: 'xidlc',
  },
  {
    workspace: 'xidlc-win32-arm64',
    archive: 'xidlc-aarch64-pc-windows-msvc.zip',
    binaryName: 'xidlc.exe',
  },
  {
    workspace: 'xidlc-win32-x64',
    archive: 'xidlc-x86_64-pc-windows-msvc.zip',
    binaryName: 'xidlc.exe',
  },
];

for (const target of targets) {
  const archivePath = path.join(releaseDir, target.archive);
  if (!fs.existsSync(archivePath)) {
    throw new Error(`Missing release archive: ${archivePath}`);
  }

  const packageDir = path.join(packagesDir, target.workspace);
  const packageJsonPath = path.join(packageDir, 'package.json');
  const manifest = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
  manifest.version = version;
  fs.writeFileSync(packageJsonPath, `${JSON.stringify(manifest, null, 2)}\n`);

  const binDir = path.join(packageDir, 'bin');
  fs.mkdirSync(binDir, { recursive: true });
  fs.rmSync(path.join(binDir, target.binaryName), { force: true });

  const tempDir = fs.mkdtempSync(
    path.join(os.tmpdir(), `xidlc-npm-${target.workspace}-`),
  );
  try {
    const extractArgs = target.archive.endsWith('.zip')
      ? ['-q', archivePath, '-d', tempDir]
      : ['-xzf', archivePath, '-C', tempDir];
    const extractCommand = target.archive.endsWith('.zip') ? 'unzip' : 'tar';
    const extractResult = spawnSync(extractCommand, extractArgs, {
      stdio: 'inherit',
    });
    if (extractResult.status !== 0) {
      throw new Error(`Failed to extract ${archivePath}`);
    }

    const extractedBinaryPath = path.join(tempDir, target.binaryName);
    if (!fs.existsSync(extractedBinaryPath)) {
      throw new Error(`Expected ${target.binaryName} in ${target.archive}`);
    }

    const destinationPath = path.join(binDir, target.binaryName);
    fs.copyFileSync(extractedBinaryPath, destinationPath);
    if (!target.binaryName.endsWith('.exe')) {
      fs.chmodSync(destinationPath, 0o755);
    }
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
}

const mainPackagePath = path.join(packagesDir, 'xidlc', 'package.json');
const mainPackage = JSON.parse(fs.readFileSync(mainPackagePath, 'utf8'));
mainPackage.version = version;
for (const dependencyName of Object.keys(mainPackage.optionalDependencies)) {
  mainPackage.optionalDependencies[dependencyName] = version;
}
fs.writeFileSync(mainPackagePath, `${JSON.stringify(mainPackage, null, 2)}\n`);

process.stdout.write(
  `staged xidlc release assets for npm version ${version}\n`,
);
