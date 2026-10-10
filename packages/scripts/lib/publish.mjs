/** Registry a publish run targets when no `--registry` is given. */
const DEFAULT_REGISTRY = 'https://registry.npmjs.org/';

/**
 * Parse the publish command line.
 *
 * Valued options accept both `--name value` and `--name=value`, so the release
 * workflow can pass `--registry=<url>` while local runs use `--name value`.
 */
export function parseArgs(argv) {
  const options = { dryRun: false, registry: DEFAULT_REGISTRY, tag: null };
  for (let index = 0; index < argv.length; index += 1) {
    const raw = argv[index];
    if (raw === '--dry-run') {
      options.dryRun = true;
      continue;
    }
    const separator = raw.indexOf('=');
    const name = separator === -1 ? raw : raw.slice(0, separator);
    if (separator === -1) {
      index += 1;
    }
    const value = separator === -1 ? argv[index] : raw.slice(separator + 1);
    if (value === undefined || value === '') {
      throw new Error(`missing value for ${name}`);
    }
    switch (name) {
      case '--registry':
        options.registry = value;
        break;
      case '--tag':
        options.tag = value;
        break;
      default:
        throw new Error(`unknown argument ${name}`);
    }
  }
  return options;
}

/**
 * Publish order of the packages: platform packages first, the wrapper last.
 *
 * The wrapper pins every platform package, so the packages it points at must
 * be on the registry before it can be installed.
 */
export function publishEntries(packages) {
  return [
    ...packages.platforms.map(platform => ({
      dir: platform.dir,
      name: platform.entry.package,
    })),
    { dir: packages.wrapperDir, name: packages.wrapper.name },
  ];
}

/**
 * Publish every entry the registry does not carry yet, best effort.
 *
 * A package that fails to publish, for example one without a trusted publisher
 * configuration, is recorded and the remaining packages are still attempted;
 * the caller reports the outcome and fails the run afterwards.
 *
 * `isPublished` and `publish` are injected so the loop stays testable without
 * touching npm.
 */
export function publishPending({ entries, version, isPublished, publish }) {
  const report = { failures: [], published: [], skipped: [] };
  for (const entry of entries) {
    try {
      if (isPublished(entry.name, version)) {
        report.skipped.push(entry.name);
        continue;
      }
      publish(entry.dir);
      report.published.push(entry.name);
    } catch (error) {
      report.failures.push({ name: entry.name, reason: error.message });
    }
  }
  return report;
}
