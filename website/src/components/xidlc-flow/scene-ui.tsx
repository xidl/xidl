import { CAPTIONS } from './constants';

export const INTRO_WINDOW_CLASS = 'left-[30px] top-[230px] h-[300px] w-[330px]';
export const WEB_WINDOW_CLASS = 'left-[680px] top-[40px] h-[310px] w-[560px]';
export const RUST_WINDOW_CLASS =
  'bottom-[20px] left-[680px] h-[310px] w-[560px]';
export const WINDOW_SHADOW_CLASS =
  'shadow-[0_12px_32px_-16px_rgb(21_23_27_/_0.18)] dark:shadow-[0_12px_32px_-16px_rgb(0_0_0_/_0.7)]';

export function Caption({ index }: { index: number }) {
  return (
    <div className="absolute bottom-[26px] left-[30px] flex items-center gap-2.5 text-sm text-flow-muted">
      <b className="font-semibold text-flow-ink">{`${index}/4`}</b>
      <span>{CAPTIONS[index - 1]}</span>
    </div>
  );
}

export function Compiler({
  ring,
  status,
}: {
  ring: 'busy' | 'idle' | 'ok';
  status: string;
}) {
  const ringClassName =
    ring === 'busy'
      ? 'border-flow-rust animate-xidlc-pulse'
      : ring === 'ok'
        ? 'border-flow-ok'
        : 'border-transparent';

  return (
    <div className="absolute left-[420px] top-[300px] flex h-[120px] w-[160px] flex-col items-center justify-center gap-1 rounded-[14px] bg-flow-ink text-flow-card">
      <div
        className={`absolute -inset-2 rounded-[20px] border-2 ${ringClassName}`}
      />
      <b className="font-flow-mono text-2xl font-semibold tracking-[-0.02em]">
        xidlc
      </b>
      <span className="text-xs opacity-65">{status}</span>
    </div>
  );
}
