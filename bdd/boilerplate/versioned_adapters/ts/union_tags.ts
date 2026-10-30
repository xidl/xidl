import assert from 'node:assert/strict';
import * as a from './versioned_union_tags.adapters';

let calls = 0;
const hooks: a.VersionAdapters = {
  deliveryDowncastToV1: source => {
    calls++;
    return { ok: true, value: { data: `attempts: ${source.data}`, tag: '0' } };
  },
  noticeDowncastToV1: source => {
    calls++;
    return { ok: true, value: { data: source.data, tag: '0' } };
  },
  noticeUpcastFromV1: source => {
    calls++;
    return { ok: true, value: { data: source.data, tag: '2' } };
  },
};

for (const value of [
  { data: 'delivered', tag: '0' },
  { data: 'already delivered', tag: '2' },
  { data: true, tag: 'default' },
] as const) {
  const result = a.decodeAndUpcastNotificationsSubmitFromV1(
    { receipt: value },
    hooks,
  );
  assert(result.ok);
  assert.deepEqual(result.value.receipt, value);
  assert.deepEqual(a.downcastDelivery('v1', result.value.receipt, hooks), {
    ok: true,
    value,
  });
}
assert.equal(calls, 0, 'mechanical copies preserve every original tag');
for (const tag of ['1', '3'] as const) {
  assert.deepEqual(a.downcastDelivery('v1', { data: 4, tag }, hooks), {
    ok: true,
    value: { data: 'attempts: 4', tag: '0' },
  });
}
assert.equal(
  calls,
  2,
  'both labels of the new case reach the same business hook',
);
for (const value of [
  { data: 4, tag: '1' },
  { data: 4, tag: '2' },
  { data: true, tag: 'other' },
  { text: 'delivered' },
  { data: 'delivered', extra: true, tag: '0' },
]) {
  assert.equal(
    a.decodeAndUpcastNotificationsSubmitFromV1({ receipt: value }, hooks).ok,
    false,
  );
}
assert.equal(calls, 2, 'invalid old wire never reaches the business hook');
const notice = a.decodeAndUpcastNotificationsReissueFromV1(
  { notice: { data: 'kept', tag: '0' } },
  hooks,
);
assert(notice.ok);
assert.deepEqual(notice.value.notice, { data: 'kept', tag: '2' });
assert.deepEqual(a.downcastNotice('v1', notice.value.notice, hooks), {
  ok: true,
  value: { data: 'kept', tag: '0' },
});
assert.equal(
  calls,
  4,
  'changing tags requires one explicit decision in each direction',
);
assert.equal(
  a.decodeAndUpcastNotificationsReissueFromV1(
    { notice: { data: 'kept', tag: '2' } },
    hooks,
  ).ok,
  false,
);
assert.equal(calls, 4);
console.log('PASS: union labels and explicit retagging');
