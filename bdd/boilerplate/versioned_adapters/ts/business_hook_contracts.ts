import type * as types from './versioned_adapters_business';
import * as a from './versioned_adapters_business.adapters';
import * as errors from './versioned_adapters_business.errors';

type Patch = types.v2.ParcelPatch;
type Head = types.v3.Parcel;
type UpdateHook = Pick<a.VersionAdapters, 'parcelPatchUpcastFromV2'>;
type UpdateDecision = a.VersionedResult<a.ParcelPatchUpcastDecisionFromV2>;

const decision = (source: Patch): UpdateDecision => ({
  ok: true,
  value: {
    handling_note: a.IGNORED,
    stored: { handling_note: source.handling_note },
  },
});

/** An application class can implement a required function property with a method. */
class UpdatePolicy implements UpdateHook {
  parcelPatchUpcastFromV2(source: Patch, _current: Head): UpdateDecision {
    return decision(source);
  }
}
export const updatePolicy: UpdateHook = new UpdatePolicy();

// A callback may accept a safe structural supertype of Head. v2 only adds an
// optional retired member, so it must not be rejected as a nominal version error.
export const structurallyCompatible: UpdateHook = {
  parcelPatchUpcastFromV2(source: Patch, _current: types.v2.Parcel) {
    return decision(source);
  },
};

// These are compiler assertions against actual generated signatures. If method
// bivariance reappears, tsc reports an unused @ts-expect-error and fails the BDD.
type NarrowHead = Head & { approvalToken: string };
const narrow = (_source: Patch, _current: NarrowHead): UpdateDecision => ({
  ok: true,
  value: { handling_note: a.IGNORED, stored: {} },
});
export const functionProperty: UpdateHook = {
  // @ts-expect-error Head does not guarantee an approval token.
  parcelPatchUpcastFromV2: narrow,
};
export const objectMethod: UpdateHook = {
  // @ts-expect-error Object methods must accept every valid Head too.
  parcelPatchUpcastFromV2(
    _source: Patch,
    _current: NarrowHead,
  ): UpdateDecision {
    return narrow(_source, _current);
  },
};
class NarrowPolicy {
  parcelPatchUpcastFromV2(source: Patch, current: NarrowHead): UpdateDecision {
    return narrow(source, current);
  }
}
// @ts-expect-error Class methods cannot require data absent from Head.
export const classMethod: UpdateHook = new NarrowPolicy();

export const narrowRequest: UpdateHook = {
  // @ts-expect-error A patch may omit the retired optional handling note.
  parcelPatchUpcastFromV2: (_source: Patch & { handling_note: string }) => ({
    ok: true,
    value: { handling_note: a.IGNORED, stored: {} },
  }),
};
export const narrowReader: Pick<a.VersionAdapters, 'parcelDowncastToV2'> = {
  // @ts-expect-error A generated reader may return undefined; no business fact is invented.
  parcelDowncastToV2: (
    _source: Head,
    _stored: { parcelHandlingNote(source: Head): string },
  ) => ({ ok: true, value: { courier_note: a.OMIT, handling_note: a.OMIT } }),
};
export const narrowError: Pick<
  a.VersionAdapters,
  'depotGetFrameworkErrorDowncastToV2'
> = {
  // @ts-expect-error A framework failure need not have this particular reason.
  depotGetFrameworkErrorDowncastToV2: (
    _source: errors.Framework & { reason: 'framework failure' },
  ) => ({
    ok: true,
    value: new errors.v2$Busy({ reason: 'framework failure' }),
  }),
};
