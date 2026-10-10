const BINARY_MAGIC = {
  darwin: [
    [0xca, 0xfe, 0xba, 0xbe],
    [0xca, 0xfe, 0xba, 0xbf],
    [0xce, 0xfa, 0xed, 0xfe],
    [0xcf, 0xfa, 0xed, 0xfe],
    [0xfe, 0xed, 0xfa, 0xce],
    [0xfe, 0xed, 0xfa, 0xcf],
  ],
  linux: [[0x7f, 0x45, 0x4c, 0x46]],
  win32: [[0x4d, 0x5a]],
};

function magicText(buffer) {
  return [...buffer.subarray(0, 4)]
    .map(byte => byte.toString(16).padStart(2, '0'))
    .join(' ');
}

/** Name of the GitHub release asset carrying a Rust target's binary. */
export function assetName(target) {
  return `xidlc-${target}.tar.gz`;
}

/** Release tag that release-please creates for an xidlc version. */
export function releaseTag(version) {
  return `v${version}`;
}

/** Download URL of one GitHub release asset. */
export function downloadUrl(options) {
  const { asset, baseUrl = 'https://github.com', repo, tag } = options;
  return `${baseUrl}/${repo}/releases/download/${tag}/${asset}`;
}

/** Parse the `owner/name` slug out of a GitHub repository URL. */
export function repositorySlug(url) {
  const match = /^https:\/\/github\.com\/([^/]+\/[^/]+?)(?:\.git)?\/?$/.exec(
    url ?? '',
  );
  if (match === null) {
    throw new Error(`cannot derive a GitHub repository from ${url}`);
  }
  return match[1];
}

/** Validate that a workflow ref names the release built for `version`. */
export function assertRefMatchesVersion(ref, version) {
  const tag = releaseTag(version);
  if (ref !== tag) {
    throw new Error(
      `ref ${ref} does not match npm version ${version}; expected ${tag}`,
    );
  }
  return tag;
}

/** Reject payloads that are not executables of `platform`'s format. */
export function assertBinaryFormat(buffer, platform) {
  const magics = BINARY_MAGIC[platform];
  if (magics === undefined) {
    throw new Error(`unsupported platform ${platform}`);
  }
  const matches = magics.some(magic =>
    magic.every((byte, index) => buffer[index] === byte),
  );
  if (!matches) {
    throw new Error(
      `expected a ${platform} executable, ` +
        `found magic bytes ${magicText(buffer)}`,
    );
  }
}
