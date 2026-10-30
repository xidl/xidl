import type { CSSProperties } from 'react';
import type { CodeLanguage, CodeSample } from './constants';
import { KEYWORDS } from './constants';

function CheckIcon() {
  return (
    <svg aria-hidden="true" height="14" viewBox="0 0 14 14" width="14">
      <path
        d="M3 7.5 L6 10.5 L11 4"
        fill="none"
        strokeLinecap="round"
        strokeLinejoin="round"
        stroke="currentColor"
        strokeWidth="2"
      />
    </svg>
  );
}

function countTypedCharacters(sample: CodeSample) {
  return sample.lines.join('').replaceAll(' ', '').length || 1;
}

function visibleLine(line: string, typedCharacters: number) {
  let visibleLength = 0;
  let remainingCharacters = typedCharacters;

  while (visibleLength < line.length && remainingCharacters > 0) {
    if (line[visibleLength] !== ' ') {
      remainingCharacters -= 1;
    }
    visibleLength += 1;
  }

  return line.slice(0, visibleLength);
}

interface CodeToken {
  className?: string;
  value: string;
}

function tokenizeCode(value: string, language: CodeLanguage) {
  const tokens: CodeToken[] = [];
  const pattern = new RegExp(
    `(\\/\\/.*$)|("[^"]*"?)|(#\\[[^\\]]*\\]?)|\\b(${KEYWORDS[language]})\\b|\\b([A-Z][A-Za-z0-9_]*|u64|string|number)\\b`,
    'g',
  );

  let lastIndex = 0;
  let match = pattern.exec(value);

  while (match) {
    if (match.index > lastIndex) {
      tokens.push({ value: value.slice(lastIndex, match.index) });
    }
    tokens.push({
      className: match[1]
        ? 'c'
        : match[2]
          ? 's'
          : match[3]
            ? 'a'
            : match[4]
              ? 'k'
              : 't',
      value: match[0],
    });
    lastIndex = pattern.lastIndex;
    match = pattern.exec(value);
  }

  if (lastIndex < value.length) {
    tokens.push({ value: value.slice(lastIndex) });
  }
  return tokens;
}

export function CodeWindow({
  code,
  progress,
  rust = false,
  className = '',
  style,
  title,
}: {
  code: CodeSample;
  progress: number;
  rust?: boolean;
  className?: string;
  style?: CSSProperties;
  title: string;
}) {
  const typedCharacters = countTypedCharacters(code);
  const completedCharacters = Math.round(progress * typedCharacters);
  const isTyping = progress < 1;
  let cumulativeCharacters = 0;
  const rows: Array<{ active: boolean; line: string }> = [];

  for (const line of code.lines) {
    const lineCharacters = line.replaceAll(' ', '').length;
    const lineProgress = completedCharacters - cumulativeCharacters;
    const isEmptyLine = line.length === 0;

    if (lineProgress <= 0 && !(isTyping && isEmptyLine)) {
      break;
    }

    if (isEmptyLine) {
      rows.push({ active: false, line: '' });
      continue;
    }

    const active = lineProgress < lineCharacters;
    rows.push({ active, line: visibleLine(line, lineProgress) });

    if (active) {
      break;
    }
    cumulativeCharacters += lineCharacters;
  }

  return (
    <div
      className={`abs win fade on${className ? ` ${className}` : ''}`}
      style={style}
    >
      <div className="bar">
        <span className="file">
          <i className={rust ? 'file-marker rust' : 'file-marker'} />
          {title}
        </span>
        <span className={rust ? 'tag rust' : 'tag'}>
          {rust ? 'Rust' : title.includes('idl') ? 'xidl' : 'Website'}
        </span>
      </div>
      <div className="code">
        {rows.map((row, lineIndex) => {
          return (
            <div className="row m-0" key={code.lines[lineIndex]}>
              <span className="ln">{lineIndex + 1}</span>
              {tokenizeCode(row.line, code.language).map(token => (
                <span className={token.className} key={token.value}>
                  {token.value}
                </span>
              ))}
              {row.active && <span className={rust ? 'caret rust' : 'caret'} />}
            </div>
          );
        })}
      </div>
      <div
        className={rust ? 'prog rust' : 'prog'}
        style={{ width: `${progress * 100}%` }}
      />
      <div className="foot">
        <span className={progress === 1 ? 'status-done' : 'status'}>
          {progress === 1 ? (
            <>
              <CheckIcon />
              {`Implemented · ${code.lines.length} lines`}
            </>
          ) : (
            'Writing…'
          )}
        </span>
        <span>{rust ? 'Rust' : 'TypeScript'}</span>
      </div>
    </div>
  );
}
