import assert from 'node:assert/strict';
import { test } from 'node:test';
import { gzipSync } from 'node:zlib';

import {
  assertBinaryFormat,
  assertRefMatchesVersion,
  assetName,
  downloadUrl,
  releaseTag,
  repositorySlug,
} from '../scripts/lib/release.mjs';
import { extractBinaryFromTarGz } from '../scripts/lib/tar.mjs';

const BLOCK = 512;

function tarHeader(name, size, options) {
  const { checksumOffset = 0, type = '0' } = options ?? {};
  const block = Buffer.alloc(BLOCK);
  block.write(name, 0, 'utf8');
  block.write('000644 ', 100, 'utf8');
  block.write('000000 ', 108, 'utf8');
  block.write('000000 ', 116, 'utf8');
  block.write(`${size.toString(8).padStart(11, '0')} `, 124, 'utf8');
  block.write('00000000000 ', 136, 'utf8');
  block.write('        ', 148, 'utf8');
  block.write(type, 156, 'utf8');
  block.write('ustar\0', 257, 'utf8');
  block.write('00', 263, 'utf8');
  let sum = checksumOffset;
  for (const byte of block) {
    sum += byte;
  }
  block.write(`${sum.toString(8).padStart(6, '0')}\0 `, 148, 'utf8');
  return block;
}

function tarGz(entries, options) {
  const blocks = [];
  for (const [name, content, type] of entries) {
    blocks.push(tarHeader(name, content.length, { ...options, type }));
    const data = Buffer.alloc(Math.ceil(content.length / BLOCK) * BLOCK);
    content.copy(data);
    blocks.push(data);
  }
  blocks.push(Buffer.alloc(BLOCK * 2));
  return gzipSync(Buffer.concat(blocks));
}

test('extractBinaryFromTarGz reads a root-level binary', () => {
  const archive = tarGz([['xidlc', Buffer.from('binary')]]);
  assert.deepEqual(
    extractBinaryFromTarGz(archive, 'xidlc'),
    Buffer.from('binary'),
  );
});

test('extractBinaryFromTarGz skips other entries and directories', () => {
  const archive = tarGz([
    ['xidlc-x86_64-unknown-linux-musl/', Buffer.alloc(0), '5'],
    ['xidlc-x86_64-unknown-linux-musl/README.md', Buffer.from('docs')],
    ['xidlc-x86_64-unknown-linux-musl/xidlc', Buffer.from('binary')],
  ]);
  assert.deepEqual(
    extractBinaryFromTarGz(archive, 'xidlc'),
    Buffer.from('binary'),
  );
});

test('extractBinaryFromTarGz follows GNU long name entries', () => {
  const archive = tarGz([
    ['./@LongLink', Buffer.from('deep/xidlc\0'), 'L'],
    ['xidlc', Buffer.from('binary')],
  ]);
  assert.deepEqual(
    extractBinaryFromTarGz(archive, 'xidlc'),
    Buffer.from('binary'),
  );
});

test('extractBinaryFromTarGz matches the executable name exactly', () => {
  const archive = tarGz([['xidlc.exe', Buffer.from('MZ')]]);
  assert.throws(
    () => extractBinaryFromTarGz(archive, 'xidlc'),
    /does not contain xidlc;/,
  );
  assert.deepEqual(
    extractBinaryFromTarGz(archive, 'xidlc.exe'),
    Buffer.from('MZ'),
  );
});

test('extractBinaryFromTarGz rejects corrupt headers', () => {
  const archive = tarGz([['xidlc', Buffer.from('binary')]], {
    checksumOffset: 1,
  });
  assert.throws(
    () => extractBinaryFromTarGz(archive, 'xidlc'),
    /corrupt tar header: checksum mismatch/,
  );
});

test('extractBinaryFromTarGz rejects archives without the binary', () => {
  const archive = tarGz([['xidlc-linux-x64/README.md', Buffer.from('docs')]]);
  assert.throws(
    () => extractBinaryFromTarGz(archive, 'xidlc'),
    /archive does not contain xidlc/,
  );
});

test('release coordinates compose', () => {
  assert.equal(
    assetName('x86_64-unknown-linux-musl'),
    'xidlc-x86_64-unknown-linux-musl.tar.gz',
  );
  assert.equal(releaseTag('0.98.0'), 'v0.98.0');
  assert.equal(
    downloadUrl({ asset: 'a.tar.gz', repo: 'xidl/xidl', tag: 'v0.98.0' }),
    'https://github.com/xidl/xidl/releases/download/v0.98.0/a.tar.gz',
  );
  assert.equal(
    downloadUrl({
      asset: 'a.tar.gz',
      baseUrl: 'https://mirror.test',
      repo: 'xidl/xidl',
      tag: 'v0.98.0',
    }),
    'https://mirror.test/xidl/xidl/releases/download/v0.98.0/a.tar.gz',
  );
});

test('repositorySlug reads GitHub repository URLs', () => {
  assert.equal(repositorySlug('https://github.com/xidl/xidl'), 'xidl/xidl');
  assert.equal(repositorySlug('https://github.com/xidl/xidl.git'), 'xidl/xidl');
  assert.equal(repositorySlug('https://github.com/xidl/xidl/'), 'xidl/xidl');
  assert.throws(
    () => repositorySlug(undefined),
    /cannot derive a GitHub repository/,
  );
  assert.throws(
    () => repositorySlug('https://gitlab.com/xidl/xidl'),
    /cannot derive a GitHub repository/,
  );
});

test('assertRefMatchesVersion guards CI refs', () => {
  assert.equal(assertRefMatchesVersion('v0.98.0', '0.98.0'), 'v0.98.0');
  assert.throws(
    () => assertRefMatchesVersion('nightly', '0.98.0'),
    /ref nightly does not match npm version 0.98.0; expected v0.98.0/,
  );
});

test('assertBinaryFormat accepts each platform format', () => {
  assertBinaryFormat(Buffer.from([0x7f, 0x45, 0x4c, 0x46, 0x02]), 'linux');
  assertBinaryFormat(Buffer.from([0xcf, 0xfa, 0xed, 0xfe]), 'darwin');
  assertBinaryFormat(Buffer.from([0xca, 0xfe, 0xba, 0xbe]), 'darwin');
  assertBinaryFormat(Buffer.from([0x4d, 0x5a, 0x90, 0x00]), 'win32');
});

test('assertBinaryFormat rejects foreign payloads', () => {
  assert.throws(
    () => assertBinaryFormat(Buffer.from('<html>'), 'linux'),
    /expected a linux executable, found magic bytes 3c 68 74 6d/,
  );
  assert.throws(
    () => assertBinaryFormat(Buffer.from([0x7f, 0x45]), 'plan9'),
    /unsupported platform plan9/,
  );
});
