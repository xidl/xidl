import { Player } from '@remotion/player';
import { DURATION_IN_FRAMES, XidlcFlowComposition } from './xidlc-flow/scenes';

export default function XidlcFlow() {
  return (
    <section aria-label="XIDL compiler workflow" className="xidlc-flow">
      <Player
        acknowledgeRemotionLicense
        autoPlay
        className="wrap"
        component={XidlcFlowComposition}
        compositionHeight={720}
        compositionWidth={1280}
        controls
        durationInFrames={DURATION_IN_FRAMES}
        fps={30}
        loop
        showVolumeControls={false}
        style={{ width: '100%' }}
      />
    </section>
  );
}
