import {
  AbsoluteFill,
  Easing,
  interpolate,
  Sequence,
  useCurrentFrame,
} from 'remotion';
import { CodeWindow } from './code';
import { CAPTIONS, RUST, WEB, XIDL } from './constants';
import { FlowWires } from './wire';

export const INTRO_DURATION_IN_FRAMES = 90;
export const COMPILE_DURATION_IN_FRAMES = 105;
export const GENERATE_DURATION_IN_FRAMES = 180;
export const RUN_DURATION_IN_FRAMES = 210;
export const DURATION_IN_FRAMES =
  INTRO_DURATION_IN_FRAMES +
  COMPILE_DURATION_IN_FRAMES +
  GENERATE_DURATION_IN_FRAMES +
  RUN_DURATION_IN_FRAMES;

function Caption({ index }: { index: number }) {
  return (
    <div className="caption">
      <b>{`${index}/4`}</b>
      <span>{CAPTIONS[index - 1]}</span>
    </div>
  );
}

export function IntroScene() {
  const frame = useCurrentFrame();
  const opacity = interpolate(frame, [0, 12], [0, 1], {
    extrapolateRight: 'clamp',
  });
  const idlProgress = interpolate(frame, [9, 63], [0, 1], {
    extrapolateLeft: 'clamp',
    extrapolateRight: 'clamp',
  });

  return (
    <AbsoluteFill className="scene">
      <CodeWindow
        className="intro-xidl"
        code={XIDL}
        progress={idlProgress}
        title="user.idl"
      />
      <div className="abs compiler fade on" style={{ left: 420, top: 300 }}>
        <div className="ring" />
        <b>xidlc</b>
        <span>waiting for input</span>
      </div>
      <Caption index={1} />
      <AbsoluteFill style={{ opacity }} />
    </AbsoluteFill>
  );
}

export function CompileScene() {
  const frame = useCurrentFrame();
  const status =
    frame < 18
      ? 'reading user.xidl'
      : frame < 60
        ? 'parsing…'
        : 'generating targets';

  return (
    <AbsoluteFill className="scene">
      <CodeWindow
        className="intro-xidl"
        code={XIDL}
        progress={1}
        title="user.idl"
      />
      <div
        className={`abs compiler fade on${frame >= 15 && frame < 90 ? ' busy' : ''}`}
        style={{ left: 420, top: 300 }}
      >
        <div className="ring" />
        <b>xidlc</b>
        <span>{status}</span>
      </div>
      <svg aria-hidden="true" className="wires" viewBox="0 0 1280 720">
        <path
          className="wire"
          d="M360 360 L420 360"
          style={{ strokeDashoffset: 0 }}
        />
      </svg>
      <Caption index={2} />
    </AbsoluteFill>
  );
}

export function GenerateScene() {
  const frame = useCurrentFrame();
  const wireProgress = interpolate(frame, [0, 24], [0, 1], {
    easing: Easing.inOut(Easing.quad),
    extrapolateRight: 'clamp',
  });
  const webProgress = interpolate(frame, [24, 120], [0, 1], {
    extrapolateLeft: 'clamp',
    extrapolateRight: 'clamp',
  });
  const rustProgress = interpolate(frame, [24, 135], [0, 1], {
    extrapolateLeft: 'clamp',
    extrapolateRight: 'clamp',
  });
  const done = frame >= 135;

  return (
    <AbsoluteFill className="scene">
      <CodeWindow
        className="intro-xidl"
        code={XIDL}
        progress={1}
        title="user.idl"
      />
      <div
        className={`abs compiler fade on${done ? ' ok' : ' busy'}`}
        style={{ left: 420, top: 300 }}
      >
        <div className="ring" />
        <b>xidlc</b>
        <span>{done ? 'all done' : 'generating targets'}</span>
      </div>
      <CodeWindow
        className="web-window"
        code={WEB}
        progress={webProgress}
        title="web/src/handler.ts"
      />
      <CodeWindow
        className="rust-window"
        code={RUST}
        progress={rustProgress}
        rust
        title="server/src/service.rs"
      />
      <svg aria-hidden="true" className="wires" viewBox="0 0 1280 720">
        <path
          className="wire"
          d="M360 360 L420 360"
          strokeDasharray="60"
          strokeDashoffset="0"
        />
        <FlowWires progress={wireProgress} />
      </svg>
      <Caption index={3} />
    </AbsoluteFill>
  );
}

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
  const doneBackground = `color-mix(in srgb, var(--xidlc-ok, #17a058) ${
    doneMorph * 100
  }%, var(--xidlc-ink, #15171b))`;
  const responseOpacity = interpolate(frame, [83, 98], [0, 1], {
    extrapolateRight: 'clamp',
  });

  return (
    <AbsoluteFill>
      <AbsoluteFill className="scene out" style={{ opacity }}>
        <CodeWindow
          className="intro-xidl"
          code={XIDL}
          progress={1}
          title="user.idl"
        />
        <div
          className="abs compiler fade on ok"
          style={{ left: 420, top: 300 }}
        >
          <div className="ring" />
          <b>xidlc</b>
          <span>all done</span>
        </div>
        <CodeWindow
          className="web-window"
          code={WEB}
          progress={1}
          title="web/src/handler.ts"
        />
        <CodeWindow
          className="rust-window"
          code={RUST}
          progress={1}
          rust
          title="server/src/service.rs"
        />
      </AbsoluteFill>
      <div className="abs win browser on" style={{ opacity: browserOpacity }}>
        <div className="chrome">
          <div className="flex gap-1">
            <i className="dot m-0 block rounded w-2 h-2 bg-[#d8dbe0]" />
            <i className="dot m-0 block rounded w-2 h-2 bg-[#d8dbe0]" />
            <i className="dot m-0 block rounded w-2 h-2 bg-[#d8dbe0]" />
          </div>
          <div className="url">localhost:3000/users/new</div>
          <div style={{ width: '46px' }} />
        </div>
        <div className="page">
          <div>
            <h2>New User</h2>
            <p className="sub">
              Business code imports the generated code and implements the
              user.xidl interface
            </p>
          </div>
          <div className="field">
            name
            <div className="input">Ferris</div>
          </div>
          <div className="btnwrap">
            <button
              className={`btn${pressed ? ' press' : ''}${loading ? ' loading' : ''}${
                done ? ' done' : ''
              }`}
              style={
                loading
                  ? { borderRadius: buttonRadius, width: buttonWidth }
                  : done
                    ? {
                        backgroundColor: doneBackground,
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
                className="label"
                style={loading ? { opacity: labelOpacity } : undefined}
              >
                Create User
              </span>
              {loading || done ? (
                <span
                  aria-hidden="true"
                  className="spinner"
                  style={{
                    opacity: loading ? spinnerOpacity : spinnerFade,
                    scale: loading ? spinnerScale : 1,
                  }}
                />
              ) : null}
              <svg
                aria-hidden="true"
                className="button-icon"
                viewBox="0 0 30 30"
              >
                <path
                  d="M7 15.5 L12.5 21 L23 9.5"
                  style={done ? { strokeDashoffset: checkDraw } : undefined}
                />
              </svg>
            </button>
          </div>
          <div className="resp on" style={{ opacity: responseOpacity }}>
            <span className="okc">200 OK</span>
            <span>{`{ id: 1, name: "Ferris" }`}</span>
          </div>
          <div
            className={`flow-cursor${pressed ? ' press' : ''}`}
            style={{
              opacity: frame >= 18 && frame < 48 ? 1 : 0,
              transform: `translate(${cursorX + 66}px, ${cursorY + 60}px)`,
            }}
          >
            <svg aria-hidden="true" viewBox="0 0 22 22">
              <path d="M3 2 L3 18 L7.5 14 L10.5 20.5 L13.2 19.3 L10.3 12.9 L16.5 12.9 Z" />
            </svg>
          </div>
        </div>
      </div>
      <Caption index={4} />
    </AbsoluteFill>
  );
}

export function XidlcFlowComposition() {
  return (
    <AbsoluteFill className="stage">
      <Sequence durationInFrames={INTRO_DURATION_IN_FRAMES}>
        <IntroScene />
      </Sequence>
      <Sequence
        durationInFrames={COMPILE_DURATION_IN_FRAMES}
        from={INTRO_DURATION_IN_FRAMES}
      >
        <CompileScene />
      </Sequence>
      <Sequence
        durationInFrames={GENERATE_DURATION_IN_FRAMES}
        from={INTRO_DURATION_IN_FRAMES + COMPILE_DURATION_IN_FRAMES}
      >
        <GenerateScene />
      </Sequence>
      <Sequence
        durationInFrames={RUN_DURATION_IN_FRAMES}
        from={
          INTRO_DURATION_IN_FRAMES +
          COMPILE_DURATION_IN_FRAMES +
          GENERATE_DURATION_IN_FRAMES
        }
      >
        <RunScene />
      </Sequence>
    </AbsoluteFill>
  );
}
