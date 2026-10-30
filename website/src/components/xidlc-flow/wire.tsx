interface WirePoint {
  x: number;
  y: number;
}

interface WireGeometry {
  path: string;
  pointAt: (progress: number) => WirePoint;
}

const WIRE_SAMPLE_COUNT = 128;

function createWireGeometry(
  start: WirePoint,
  controlStart: WirePoint,
  controlEnd: WirePoint,
  end: WirePoint,
): WireGeometry {
  const samples = Array.from({ length: WIRE_SAMPLE_COUNT + 1 }, (_, index) => {
    const time = index / WIRE_SAMPLE_COUNT;
    const inverse = 1 - time;
    const inverseSquared = inverse * inverse;
    const timeSquared = time * time;
    const startWeight = inverseSquared * inverse;
    const controlStartWeight = 3 * inverseSquared * time;
    const controlEndWeight = 3 * inverse * timeSquared;
    const endWeight = timeSquared * time;

    return {
      x:
        start.x * startWeight +
        controlStart.x * controlStartWeight +
        controlEnd.x * controlEndWeight +
        end.x * endWeight,
      y:
        start.y * startWeight +
        controlStart.y * controlStartWeight +
        controlEnd.y * controlEndWeight +
        end.y * endWeight,
    };
  });

  const cumulativeLengths = [0];
  for (let index = 1; index < samples.length; index += 1) {
    const previous = samples[index - 1];
    const current = samples[index];
    const segmentLength = Math.hypot(
      current.x - previous.x,
      current.y - previous.y,
    );
    cumulativeLengths.push(cumulativeLengths[index - 1] + segmentLength);
  }

  const totalLength = cumulativeLengths[cumulativeLengths.length - 1];

  return {
    path: `M${start.x} ${start.y} C${controlStart.x} ${controlStart.y} ${controlEnd.x} ${controlEnd.y} ${end.x} ${end.y}`,
    pointAt(progress) {
      const clamped = Math.min(1, Math.max(0, progress));
      const targetLength = clamped * totalLength;
      let segmentIndex = 0;

      while (
        segmentIndex < samples.length - 2 &&
        cumulativeLengths[segmentIndex + 1] < targetLength
      ) {
        segmentIndex += 1;
      }

      const segmentStart = samples[segmentIndex];
      const segmentEnd = samples[segmentIndex + 1];
      const segmentLength =
        cumulativeLengths[segmentIndex + 1] - cumulativeLengths[segmentIndex];
      const segmentProgress =
        segmentLength === 0
          ? 0
          : (targetLength - cumulativeLengths[segmentIndex]) / segmentLength;

      return {
        x: segmentStart.x + (segmentEnd.x - segmentStart.x) * segmentProgress,
        y: segmentStart.y + (segmentEnd.y - segmentStart.y) * segmentProgress,
      };
    },
  };
}

const WEB_WIRE = createWireGeometry(
  { x: 580, y: 360 },
  { x: 632, y: 360 },
  { x: 628, y: 195 },
  { x: 680, y: 195 },
);

const RUST_WIRE = createWireGeometry(
  { x: 580, y: 360 },
  { x: 632, y: 360 },
  { x: 628, y: 535 },
  { x: 680, y: 535 },
);

function Wire({
  className = '',
  geometry,
  progress,
}: {
  className?: string;
  geometry: WireGeometry;
  progress: number;
}) {
  const dot = geometry.pointAt(progress);
  const dotClassName = className ? `wire-dot ${className}` : 'wire-dot';
  const wireClassName = className ? `wire ${className}` : 'wire';

  return (
    <>
      <path
        className={wireClassName}
        d={geometry.path}
        pathLength={1}
        strokeDasharray="1"
        strokeDashoffset={1 - progress}
      />
      <circle className={dotClassName} cx={dot.x} cy={dot.y} r="5" />
    </>
  );
}

export function FlowWires({ progress }: { progress: number }) {
  return (
    <>
      <Wire geometry={WEB_WIRE} progress={progress} />
      <Wire className="rust" geometry={RUST_WIRE} progress={progress} />
    </>
  );
}
