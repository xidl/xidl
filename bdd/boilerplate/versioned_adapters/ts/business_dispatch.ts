import assert from 'node:assert/strict';
import * as a from './versioned_adapters_business.adapters';
import * as errors from './versioned_adapters_business.errors';

const take = <T>(
  result: { ok: true; value: T } | { ok: false; error: unknown },
): T => {
  if (!result.ok) throw new Error(JSON.stringify(result.error));
  return result.value;
};

const refuse = (
  version: string,
  typePath: string,
): a.VersionedResult<never> => ({
  error: {
    direction: 'downcast',
    reason: 'outcome cannot be represented',
    typePath,
    version,
  },
  ok: false,
});

/**
 * Dispatch-level boundaries: the union variant the oldest contract never
 * declared, the error-downgrade table, and refusals that must retain the
 * committed Head outcome instead of projecting a substitute.
 */
export async function runDispatchJourney(
  hooks: a.VersionAdapters,
  storedValues: a.StoredValues,
  wireParcel: Record<string, unknown>,
  counters: { calls: () => number; errorHooks: () => number },
): Promise<void> {
  // The newest union variant still answers the oldest caller: the count the
  // oldest contract never declared reaches it as text through the hook, the
  // declared variant passes mechanically, and a refusal keeps the Head outcome.
  const counted = take(
    a.decodeAndUpcastDepotChooseFromV2(
      { choice: { data: 7, tag: '1' } },
      hooks,
    ),
  ).choice;
  assert.deepEqual(take(a.downcastChoice('v1', counted, hooks)), {
    data: '7',
    tag: '0',
  });
  assert.deepEqual(
    take(a.downcastChoice('v1', { data: 'kept', tag: '0' }, hooks)),
    { data: 'kept', tag: '0' },
  );
  const chosen = await a.dispatchDepotChooseFromV1(
    { choice: { data: 'seed', tag: '0' } },
    async () => ({ ok: true, value: counted }),
    hooks,
  );
  assert(chosen.dispatched && chosen.projected && chosen.value.ok);
  assert.deepEqual(chosen.value.value, { data: '7', tag: '0' });
  const refusedChoice = await a.dispatchDepotChooseFromV1(
    { choice: { data: 'seed', tag: '0' } },
    async () => ({ ok: true, value: counted }),
    { ...hooks, choiceDowncastToV1: () => refuse('2026-01', 'Choice') },
  );
  assert.equal(refusedChoice.dispatched, true);
  assert.equal(refusedChoice.projected, false);
  if (!refusedChoice.projected) {
    assert.equal(
      (refusedChoice.head as { ok: true; value: typeof counted }).value,
      counted,
      'the refusal retains the committed Head outcome',
    );
  }

  const framework: a.FrameworkError = {
    error: new Error('transport'),
    kind: 'Framework',
  };
  assert.equal(
    take(a.downcastDepotGetErrorToV1(framework, storedValues, hooks)),
    framework,
  );
  assert.equal(counters.errorHooks(), 0);
  const businessError = take(
    a.downcastDepotGetErrorToV1(
      new errors.Framework({ reason: 'locked' }),
      storedValues,
      hooks,
    ),
  );
  assert(businessError instanceof errors.v1$NotFound);
  assert.equal(businessError.id, 'locked');
  assert.equal(counters.errorHooks(), 2);
  const denied = take(
    a.downcastDepotGetErrorToV1(
      new errors.Denied({ reason: 'denied' }),
      storedValues,
      hooks,
    ),
  );
  assert(denied instanceof errors.v1$NotFound);
  assert.equal(
    counters.errorHooks(),
    4,
    'every adjacent error hook must execute exactly once',
  );

  const typedHead = {
    error: new errors.Denied({ reason: 'original denial' }),
    ok: false as const,
  };
  const typedRefusal = a.downcastDepotGetOutcomeToV1(typedHead, storedValues, {
    ...hooks,
    depotGetBusyErrorDowncastToV1: () => refuse('2026-01', 'Depot.get'),
  });
  assert.equal(typedRefusal.projected, false);
  if (!typedRefusal.projected) {
    assert.equal(typedRefusal.head, typedHead);
    assert.equal(typedRefusal.head.error.reason, 'original denial');
  }

  const terminal = { ok: true as const, value: 'delivered' as const };
  const failure = a.downcastDepotShipOutcomeToV1(terminal, hooks);
  assert.equal(failure.projected, false);
  if (!failure.projected) {
    assert.equal(failure.head, terminal);
    assert.equal(
      failure.error.version,
      '2026-01',
      'later downcast refusal must retain the original delivered outcome',
    );
  }
  const blocked: a.VersionAdapters = {
    ...hooks,
    parcelUpcastFromV2: () => ({
      error: {
        direction: 'upcast',
        reason: 'business decision refused',
        typePath: 'Parcel',
        version: '2026-04',
      },
      ok: false,
    }),
  };
  const beforeMiddleRefusal = counters.calls();
  let callsAfterRefusal = 0;
  const rejected = await a.dispatchDepotShip(
    'v1',
    { parcel: wireParcel },
    async () => {
      callsAfterRefusal++;
      return { ok: true, value: 'pending' };
    },
    blocked,
  );
  assert.equal(rejected.dispatched, false);
  assert.equal(callsAfterRefusal, 0);
  assert.equal(
    counters.calls(),
    beforeMiddleRefusal + 1,
    'the earlier upcast ran before the middle refusal',
  );
  const headOnly = await a.dispatchDepotHeadOnly(
    'v3',
    {},
    async (_, context) => {
      assert.equal(context.originalVersionId, 'v3');
      return { ok: true, value: 'head' };
    },
    hooks,
  );
  assert.equal(
    headOnly.dispatched &&
      headOnly.projected &&
      headOnly.value.ok &&
      headOnly.value.value,
    'head',
  );
}
