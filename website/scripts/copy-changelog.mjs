import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const websiteDir = join(dirname(fileURLToPath(import.meta.url)), '..');

const changelog = readFileSync(join(websiteDir, '..', 'CHANGELOG.md'), 'utf8');
const body = changelog.replace(/^# Changelog\s*\n+/, '');

const target = join(websiteDir, 'src', 'content', 'docs', 'changelog.md');
mkdirSync(dirname(target), { recursive: true });
writeFileSync(
  target,
  `---
title: Changelog
sidebar:
  hidden: true
---

${body}`,
);
