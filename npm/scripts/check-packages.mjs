import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';

const rootDir = path.resolve(import.meta.dirname, '..');
const workspacePackagePath = path.join(rootDir, 'package.json');
const workspacePackage = JSON.parse(
  fs.readFileSync(workspacePackagePath, 'utf8'),
);
const expectedVersion = workspacePackage.version;
const packagesDir = path.join(rootDir, 'packages');
const packageNames = fs.readdirSync(packagesDir).filter(name => {
  const packagePath = path.join(packagesDir, name);
  return fs.statSync(packagePath).isDirectory();
});

for (const packageName of packageNames) {
  const packageJsonPath = path.join(packagesDir, packageName, 'package.json');
  const manifest = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
  if (manifest.version !== expectedVersion) {
    throw new Error(
      `${manifest.name} has version ${manifest.version}, expected ${expectedVersion}`,
    );
  }

  const optionalDependencies = manifest.optionalDependencies ?? {};
  for (const [dependencyName, version] of Object.entries(
    optionalDependencies,
  )) {
    if (version !== expectedVersion) {
      throw new Error(
        `${manifest.name} optional dependency ${dependencyName} uses ${version}, expected ${expectedVersion}`,
      );
    }
  }
}

process.stdout.write(
  `npm package manifests are aligned at version ${expectedVersion}\n`,
);
