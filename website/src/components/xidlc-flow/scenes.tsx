import {
  AbsoluteFill,
  Easing,
  interpolate,
  Sequence,
  useCurrentFrame,
} from 'remotion';
import { CodeWindow } from './code';
import { RUST, WEB, XIDL } from './constants';
import { RunScene } from './run-scene';
import {
  Caption,
  Compiler,
  INTRO_WINDOW_CLASS,
  RUST_WINDOW_CLASS,
  WEB_WINDOW_CLASS,
} from './scene-ui';
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
    <AbsoluteFill>
      <CodeWindow
        className={INTRO_WINDOW_CLASS}
        code={XIDL}
        progress={idlProgress}
        title="user.idl"
      />
      <Compiler ring="idle" status="waiting for input" />
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
    <AbsoluteFill>
      <CodeWindow
        className={INTRO_WINDOW_CLASS}
        code={XIDL}
        progress={1}
        title="user.idl"
      />
      <Compiler
        ring={frame >= 15 && frame < 90 ? 'busy' : 'idle'}
        status={status}
      />
      <svg
        aria-hidden="true"
        className="pointer-events-none absolute inset-0 overflow-visible"
        viewBox="0 0 1280 720"
      >
        <path
          className="fill-none stroke-flow-muted stroke-2 [stroke-linecap:round]"
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
    <AbsoluteFill>
      <CodeWindow
        className={INTRO_WINDOW_CLASS}
        code={XIDL}
        progress={1}
        title="user.idl"
      />
      <Compiler
        ring={done ? 'ok' : 'busy'}
        status={done ? 'all done' : 'generating targets'}
      />
      <CodeWindow
        className={WEB_WINDOW_CLASS}
        code={WEB}
        progress={webProgress}
        title="web/src/handler.ts"
      />
      <CodeWindow
        className={RUST_WINDOW_CLASS}
        code={RUST}
        progress={rustProgress}
        rust
        title="server/src/service.rs"
      />
      <svg
        aria-hidden="true"
        className="pointer-events-none absolute inset-0 overflow-visible"
        viewBox="0 0 1280 720"
      >
        <path
          className="fill-none stroke-flow-muted stroke-2 [stroke-linecap:round]"
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

export function XidlcFlowComposition() {
  return (
    <AbsoluteFill className="font-flow text-flow-ink">
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
