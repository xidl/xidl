#!/usr/bin/env node
import { chmodSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { dirname } from 'node:path';

import {
  assertBinaryFormat,
  assertRefMatchesVersion,
  assetName,
  downloadUrl,
  releaseTag,
  repositorySlug,
} from './lib/release.mjs';
import { extractBinaryFromTarGz } from './lib/tar.mjs';
import { binaryPath, readPackages } from './lib/workspace.mjs';

const DEFAULT_BASE_URL = 'https://github.com';

function parseArgs(argv) {
  const options = {
    baseUrl: DEFAULT_BASE_URL,
    ref: null,
    repo: null,
    version: null,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const name = argv[index];
    const value = argv[index + 1];
    if (value === undefined) {
      throw new Error(`missing value for ${name}`);
    }
    switch (name) {
      case '--base-url':
        options.baseUrl = value;
        break;
      case '--ref':
        options.ref = value;
        break;
      case '--repo':
        options.repo = value;
        break;
      case '--version':
        options.version = value;
        break;
      default:
        throw new Error(`unknown argument ${name}`);
    }
    index += 1;
  }
  return options;
}

async function download(url) {
  const headers = {};
  const token = process.env.GITHUB_TOKEN;
  if (token !== undefined && token !== '') {
    headers.authorization = `Bearer ${token}`;
  }
  const response = await fetch(url, { headers });
  if (!response.ok) {
    throw new Error(
      `cannot download ${url}: HTTP ${response.status} ${response.statusText}`,
    );
  }
  return Buffer.from(await response.arrayBuffer());
}

function place(binary, platform, asset) {
  const target = binaryPath(platform);
  rmSync(dirname(target), { force: true, recursive: true });
  mkdirSync(dirname(target), { recursive: true });
  writeFileSync(target, binary);
  chmodSync(target, 0o755);
  console.log(
    `${platform.entry.package}: ${binary.length} bytes from ${asset}`,
  );
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  const packages = readPackages();
  const version = options.version ?? packages.wrapper.version;
  const tag =
    options.ref === null
      ? releaseTag(version)
      : assertRefMatchesVersion(options.ref, version);
  const repo = options.repo ?? repositorySlug(packages.wrapper.repository?.url);
  console.log(`fetching xidlc ${version} from ${repo} at ${tag}`);
  for (const platform of packages.platforms) {
    const asset = assetName(platform.entry.target);
    const archive = await download(
      downloadUrl({ asset, baseUrl: options.baseUrl, repo, tag }),
    );
    const binary = extractBinaryFromTarGz(archive, platform.entry.binary);
    assertBinaryFormat(binary, platform.platformKey.split('-')[0]);
    place(binary, platform, asset);
  }
  console.log('binaries in place; publish with: just npm-publish');
}

main().catch(error => {
  console.error(error.message);
  process.exitCode = 1;
});
