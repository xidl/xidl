import type { PropItem } from '@/components/playground/types';

export function buildProps(items: PropItem[]) {
  const props: Record<string, any> = {};
  for (const item of items) {
    const key = item.key.trim();
    if (!key) continue;

    const raw = item.value.trim();
    if (!raw) {
      props[key] = '';
      continue;
    }

    try {
      props[key] = JSON.parse(raw);
    } catch {
      props[key] = raw;
    }
  }
  return props;
}

export function parseUrlBoolean(value: string | null) {
  if (value === null) return null;
  if (value === '1' || value === 'true') return true;
  if (value === '0' || value === 'false') return false;
  return null;
}

export function inferLanguage(
  language: string,
  path: string | undefined,
): string {
  if (language === 'hir') {
    return 'hir';
  }
  if (language === 'typed_ast') {
    return 'typed_ast';
  }
  if (!path) return 'text';

  const ext = path.split('.').pop()?.toLowerCase();
  const langMap: Record<string, string> = {
    rs: 'rust',
    c: 'c',
    cpp: 'cpp',
    cc: 'cpp',
    h: 'c',
    hpp: 'cpp',
    json: 'json',
    toml: 'toml',
    yaml: 'yaml',
    yml: 'yaml',
    ts: 'typescript',
  };
  return langMap[ext || ''] || 'text';
}
