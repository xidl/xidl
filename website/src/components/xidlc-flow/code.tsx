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
        ? 'text-flow-muted'
        : match[2]
          ? 'text-flow-ok'
          : match[3]
            ? 'text-flow-muted'
            : match[4]
              ? 'text-flow-rust'
              : 'font-semibold',
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
      className={`absolute overflow-hidden rounded-xl border border-flow-line bg-flow-card shadow-[0_12px_32px_-16px_rgb(21_23_27_/_0.18)] dark:shadow-[0_12px_32px_-16px_rgb(0_0_0_/_0.7)]${className ? ` ${className}` : ''}`}
      style={style}
    >
      <div className="flex h-10 items-center justify-between border-b border-flow-line px-[14px] font-flow-mono text-[12.5px] text-flow-muted">
        <span className="flex items-center gap-2 text-flow-ink">
          <i
            className={`inline-block h-2 w-2 rounded-[2px] ${
              rust ? 'bg-flow-rust' : 'bg-flow-ink'
            }`}
          />
          {title}
        </span>
        <span
          className={`rounded-[5px] border px-2 py-[3px] font-flow text-[11.5px] font-semibold uppercase tracking-[0.04em] ${
            rust
              ? 'border-flow-rust/35 bg-flow-rust/[0.07] text-flow-rust'
              : 'border-flow-line text-flow-ink'
          }`}
        >
          {rust ? 'Rust' : title.includes('idl') ? 'xidl' : 'Website'}
        </span>
      </div>
      <div className="absolute inset-x-0 bottom-[34px] top-10 m-0 min-h-0 overflow-auto whitespace-pre py-3 font-flow-mono text-[12.5px] leading-5">
        {rows.map((row, lineIndex) => {
          return (
            <div className="m-0 flex min-h-5" key={code.lines[lineIndex]}>
              <span className="w-10 flex-none select-none pr-[14px] text-right text-flow-line">
                {lineIndex + 1}
              </span>
              {tokenizeCode(row.line, code.language).map(token => (
                <span className={token.className} key={token.value}>
                  {token.value}
                </span>
              ))}
              {row.active && (
                <span
                  className={`mt-[3px] inline-block h-[15px] w-[7px] animate-xidlc-blink motion-reduce:animate-none ${
                    rust ? 'bg-flow-rust' : 'bg-flow-ink'
                  }`}
                />
              )}
            </div>
          );
        })}
      </div>
      <div
        className={`absolute bottom-[34px] left-0 h-0.5 ${
          rust ? 'bg-flow-rust' : 'bg-flow-ink'
        }`}
        style={{ width: `${progress * 100}%` }}
      />
      <div className="absolute inset-x-0 bottom-0 flex h-[34px] items-center justify-between border-t border-flow-line bg-flow-card px-[14px] text-xs text-flow-muted">
        <span
          className={
            progress === 1
              ? 'flex items-center gap-1.5 font-semibold text-flow-ok'
              : 'flex items-center gap-1.5'
          }
        >
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
