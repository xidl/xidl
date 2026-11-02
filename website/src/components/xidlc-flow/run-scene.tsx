import { AbsoluteFill, Easing, interpolate, useCurrentFrame } from 'remotion';
import { CodeWindow } from './code';
import { RUST, WEB, XIDL } from './constants';
import {
  Caption,
  Compiler,
  INTRO_WINDOW_CLASS,
  RUST_WINDOW_CLASS,
  WEB_WINDOW_CLASS,
  WINDOW_SHADOW_CLASS,
} from './scene-ui';

export function RunScene() {
  const frame = useCurrentFrame();
  const opacity = interpolate(frame, [0, 18], [1, 0], {
    extrapolateRight: 'clamp',
  });
  const browserOpacity = interpolate(frame, [0, 18], [0, 1], {
    extrapolateRight: 'clamp',
  });
  const cursorX = interpolate(frame, [18, 48], [620, 330], {
    easing: Easing.inOut(Easing.cubic),
    extrapolateLeft: 'clamp',
    extrapolateRight: 'clamp',
  });
  const cursorY = interpolate(frame, [18, 48], [320, 249], {
    easing: Easing.inOut(Easing.cubic),
    extrapolateLeft: 'clamp',
    extrapolateRight: 'clamp',
  });
  const pressed = frame >= 48 && frame < 53;
  const loading = frame >= 53 && frame < 83;
  const done = frame >= 83;
  const morph = interpolate(frame, [53, 68], [0, 1], {
    easing: Easing.inOut(Easing.cubic),
    extrapolateLeft: 'clamp',
    extrapolateRight: 'clamp',
  });
  const buttonWidth = interpolate(morph, [0, 1], [340, 48]);
  const buttonRadius = interpolate(morph, [0, 1], [10, 24]);
  const labelOpacity = interpolate(frame, [53, 57], [1, 0], {
    extrapolateLeft: 'clamp',
    extrapolateRight: 'clamp',
  });
  const spinnerOpacity = interpolate(frame, [54, 60], [0, 1], {
    extrapolateLeft: 'clamp',
    extrapolateRight: 'clamp',
  });
  const spinnerScale = interpolate(frame, [54, 62], [0.6, 1], {
    easing: Easing.out(Easing.cubic),
    extrapolateLeft: 'clamp',
    extrapolateRight: 'clamp',
  });
  const doneMorph = interpolate(frame, [83, 94], [0, 1], {
    easing: Easing.inOut(Easing.cubic),
    extrapolateLeft: 'clamp',
    extrapolateRight: 'clamp',
  });
  const doneWidth = interpolate(doneMorph, [0, 1], [48, 64]);
  const doneHeight = interpolate(doneMorph, [0, 1], [48, 64]);
  const doneRadius = interpolate(doneMorph, [0, 1], [24, 32]);
  const spinnerFade = interpolate(frame, [83, 89], [1, 0], {
    extrapolateLeft: 'clamp',
    extrapolateRight: 'clamp',
  });
  const checkDraw = interpolate(frame, [86, 96], [30, 0], {
    easing: Easing.inOut(Easing.cubic),
    extrapolateLeft: 'clamp',
    extrapolateRight: 'clamp',
  });
  const responseOpacity = interpolate(frame, [83, 98], [0, 1], {
    extrapolateRight: 'clamp',
  });

  return (
    <AbsoluteFill>
      <AbsoluteFill className="scale-[0.94] blur-[4px]" style={{ opacity }}>
        <CodeWindow
          className={INTRO_WINDOW_CLASS}
          code={XIDL}
          progress={1}
          title="user.idl"
        />
        <Compiler ring="ok" status="all done" />
        <CodeWindow
          className={WEB_WINDOW_CLASS}
          code={WEB}
          progress={1}
          title="web/src/handler.ts"
        />
        <CodeWindow
          className={RUST_WINDOW_CLASS}
          code={RUST}
          progress={1}
          rust
          title="server/src/service.rs"
        />
      </AbsoluteFill>
      <div
        className={`absolute left-[180px] top-[90px] h-[560px] w-[800px] overflow-hidden rounded-xl border border-flow-line bg-flow-card ${WINDOW_SHADOW_CLASS}`}
        style={{ opacity: browserOpacity }}
      >
        <div className="flex h-11 items-center gap-3.5 border-b border-flow-line px-4">
          <div className="flex gap-1">
            <i className="m-0 block h-2 w-2 rounded bg-flow-line" />
            <i className="m-0 block h-2 w-2 rounded bg-flow-line" />
            <i className="m-0 block h-2 w-2 rounded bg-flow-line" />
          </div>
          <div className="mx-auto flex h-[26px] max-w-[360px] flex-1 items-center justify-center rounded-[7px] bg-flow-bg font-flow-mono text-xs text-flow-muted">
            localhost:3000/users/new
          </div>
          <div style={{ width: '46px' }} />
        </div>
        <div className="relative flex h-[calc(100%-44px)] flex-col items-center justify-center gap-[22px]">
          <div>
            <h2 className="text-center text-[26px] font-semibold tracking-[-0.02em]">
              New User
            </h2>
            <p className="mt-1.5 text-center text-sm leading-normal text-flow-muted">
              Business code imports the generated code and implements the
              user.xidl interface
            </p>
          </div>
          <div className="flex w-[340px] flex-col gap-1.5 text-[13px] text-flow-muted">
            name
            <div className="flex h-11 items-center rounded-[10px] border border-flow-line px-[14px] text-[15px] text-flow-ink">
              Ferris
            </div>
          </div>
          <div className="flex h-16 items-center justify-center">
            <button
              className={`relative flex h-12 w-[340px] items-center justify-center rounded-[10px] bg-flow-ink text-[15px] font-semibold text-flow-card [font:inherit] ${
                pressed ? 'scale-[0.97]' : ''
              }`}
              style={
                loading
                  ? { borderRadius: buttonRadius, width: buttonWidth }
                  : done
                    ? {
                        borderRadius: doneRadius,
                        height: doneHeight,
                        width: doneWidth,
                      }
                    : undefined
              }
              tabIndex={-1}
              type="button"
            >
              <span
                aria-hidden="true"
                className="absolute inset-0 rounded-[inherit] bg-flow-ok"
                style={{ opacity: done ? doneMorph : 0 }}
              />
              <span
                className={`relative z-10 ${done ? 'invisible' : ''}`}
                style={loading ? { opacity: labelOpacity } : undefined}
              >
                Create User
              </span>
              {loading || done ? (
                <span
                  aria-hidden="true"
                  className="absolute z-10 h-5 w-5 animate-xidlc-spin rounded-full border-2 border-flow-card/30 border-t-flow-card"
                  style={{
                    opacity: loading ? spinnerOpacity : spinnerFade,
                    scale: loading ? spinnerScale : 1,
                  }}
                />
              ) : null}
              <svg
                aria-hidden="true"
                className="absolute z-10 h-[30px] w-[30px]"
                viewBox="0 0 30 30"
              >
                <path
                  className="fill-none stroke-flow-card stroke-[3.2] [stroke-dasharray:30] [stroke-dashoffset:30] [stroke-linecap:round] [stroke-linejoin:round]"
                  d="M7 15.5 L12.5 21 L23 9.5"
                  style={done ? { strokeDashoffset: checkDraw } : undefined}
                />
              </svg>
            </button>
          </div>
          <div
            className="flex h-[22px] items-center gap-2 font-flow-mono text-[13px] text-flow-muted"
            style={{ opacity: responseOpacity }}
          >
            <span className="font-semibold text-flow-ok">200 OK</span>
            <span>{`{ id: 1, name: "Ferris" }`}</span>
          </div>
          <div
            className="pointer-events-none absolute left-0 top-0 z-[5] h-[22px] w-[22px]"
            style={{
              opacity: frame >= 18 && frame < 48 ? 1 : 0,
              transform: `translate(${cursorX + 66}px, ${cursorY + 60}px)`,
            }}
          >
            <svg
              aria-hidden="true"
              className={`h-[22px] w-[22px] origin-[3px_3px] ${
                pressed ? 'scale-[0.82]' : ''
              }`}
              viewBox="0 0 22 22"
            >
              <path
                className="fill-flow-ink stroke-flow-card stroke-[1.4]"
                d="M3 2 L3 18 L7.5 14 L10.5 20.5 L13.2 19.3 L10.3 12.9 L16.5 12.9 Z"
              />
            </svg>
          </div>
        </div>
      </div>
      <Caption index={4} />
    </AbsoluteFill>
  );
}
