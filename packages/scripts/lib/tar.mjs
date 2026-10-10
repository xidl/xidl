import { basename } from 'node:path';
import { gunzipSync } from 'node:zlib';

const BLOCK_SIZE = 512;
const CHECKSUM_OFFSET = 148;
const CHECKSUM_LENGTH = 8;
const NAME_LENGTH = 100;
const PREFIX_LENGTH = 155;
const PREFIX_OFFSET = 345;
const SIZE_LENGTH = 12;
const SIZE_OFFSET = 124;
const TYPE_OFFSET = 156;
const LONG_NAME_TYPE = 'L';
const REGULAR_FILE_TYPES = new Set(['\0', '0']);

function fieldText(block, offset, length) {
  let end = block.indexOf(0, offset);
  if (end === -1 || end > offset + length) {
    end = offset + length;
  }
  return block.subarray(offset, end).toString('utf8').trim();
}

function octalValue(block, offset, length) {
  const text = fieldText(block, offset, length);
  const value = Number.parseInt(text, 8);
  if (!Number.isInteger(value) || value < 0) {
    throw new Error(
      `corrupt tar header: ${JSON.stringify(text)} at byte ${offset} is not octal`,
    );
  }
  return value;
}

function checksumMatches(block) {
  const header = Buffer.from(block.subarray(0, BLOCK_SIZE));
  const expected = octalValue(block, CHECKSUM_OFFSET, CHECKSUM_LENGTH);
  header.fill(0x20, CHECKSUM_OFFSET, CHECKSUM_OFFSET + CHECKSUM_LENGTH);
  let unsigned = 0;
  let signed = 0;
  for (const byte of header) {
    unsigned += byte;
    signed += byte < 128 ? byte : byte - 256;
  }
  return unsigned === expected || signed === expected;
}

function isZeroBlock(block) {
  return block.every(byte => byte === 0);
}

function entryName(header, longName) {
  if (longName !== null) {
    return longName;
  }
  const name = fieldText(header, 0, NAME_LENGTH);
  const prefix = fieldText(header, PREFIX_OFFSET, PREFIX_LENGTH);
  return prefix === '' ? name : `${prefix}/${name}`;
}

function longNameOf(data) {
  return data.toString('utf8').replace(/\0[\s\S]*$/, '');
}

/**
 * Extract one file from a gzipped tar archive.
 *
 * Release archives hold a single executable at the archive root, so the entry
 * is matched by base name and directory prefixes are accepted.
 */
export function extractBinaryFromTarGz(archive, binaryName) {
  const tar = gunzipSync(archive);
  const entries = [];
  let longName = null;
  let offset = 0;
  while (offset + BLOCK_SIZE <= tar.length) {
    const header = tar.subarray(offset, offset + BLOCK_SIZE);
    if (isZeroBlock(header)) {
      break;
    }
    if (!checksumMatches(header)) {
      throw new Error('corrupt tar header: checksum mismatch');
    }
    const size = octalValue(header, SIZE_OFFSET, SIZE_LENGTH);
    const dataOffset = offset + BLOCK_SIZE;
    const data = tar.subarray(dataOffset, dataOffset + size);
    const type = String.fromCharCode(header[TYPE_OFFSET]);
    if (type === LONG_NAME_TYPE) {
      longName = longNameOf(data);
    } else {
      if (REGULAR_FILE_TYPES.has(type)) {
        const name = entryName(header, longName);
        entries.push(name);
        if (basename(name) === binaryName) {
          return Buffer.from(data);
        }
      }
      longName = null;
    }
    offset = dataOffset + Math.ceil(size / BLOCK_SIZE) * BLOCK_SIZE;
  }
  throw new Error(
    `archive does not contain ${binaryName}; entries: ${entries.join(', ') || 'none'}`,
  );
}
