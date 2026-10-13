import { test } from 'node:test';
import assert from 'node:assert/strict';
import { matchesEngineEvent, applyInstanceEvent, reconcileInstanceSnapshot, createEngineEventBatcher } from '../src/shared/lib/engine-events.ts';
import { setTimeout as sleep } from 'node:timers/promises';

const instance = { id: 'a', definition_key: 'def', state: 'Running', current_node: 'start',
  variables: { value: 1 }, audit_log: [], business_key: '', started_at: null, completed_at: null };
const change = { type: 'instance_changed', instance_id: 'a', definition_key: 'def',
  delta: { state: 'Completed', current_node: 'end', business_key: '', started_at: null, completed_at: 'now' } };

test('scoped events skip other instances and definitions', () => {
  assert.equal(matchesEngineEvent(change, undefined, { instanceId: 'b' }), false);
  assert.equal(matchesEngineEvent(change, undefined, { definitionKey: 'other' }), false);
  assert.equal(matchesEngineEvent(change, ['task_changed']), false);
  assert.equal(matchesEngineEvent(change, ['instance_changed'], { instanceId: 'a' }), true);
});

test('legacy events and resync retain full-refresh fallback', () => {
  assert.equal(matchesEngineEvent({ type: 'instance_changed' }, undefined, { instanceId: 'a' }), true);
  assert.equal(matchesEngineEvent({ type: 'resync_required' }, ['task_changed'], { instanceId: 'b' }), true);
  assert.equal(applyInstanceEvent([instance], { type: 'instance_changed' }), null);
});

test('compact delta updates one row without mutating other data', () => {
  const other = { ...instance, id: 'b' };
  const result = applyInstanceEvent([instance, other], change);
  assert.equal(result[0].state, 'Completed');
  assert.equal(result[0].variables, instance.variables);
  assert.equal(result[1], other);
  assert.equal(instance.state, 'Running');
});

test('delete delta removes only the identified row', () => {
  assert.deepEqual(applyInstanceEvent([instance], { ...change, removed: true, delta: undefined }), []);
  assert.equal(applyInstanceEvent([], change), null);
});

test('reload does not overwrite a newer SSE delta or resurrect deleted rows', () => {
  const current = applyInstanceEvent([instance], change);
  assert.equal(reconcileInstanceSnapshot([instance], current, [change])[0].state, 'Completed');
  assert.deepEqual(reconcileInstanceSnapshot([instance], [], [{ ...change, removed: true }]), []);
});

test('a new instance fetched during reload is retained', () => {
  assert.equal(reconcileInstanceSnapshot([], [instance], [change])[0].state, 'Completed');
});

test('a burst causes one aggregate refresh and resync wins over later deltas', async () => {
  const delivered = [];
  const batcher = createEngineEventBatcher(event => delivered.push(event), () => 10);
  for (let i = 0; i < 100; i++) batcher.push(change);
  batcher.push({ type: 'resync_required' });
  batcher.push(change);
  await sleep(30);
  assert.deepEqual(delivered, [{ type: 'resync_required' }]);
  batcher.push(change);
  await sleep(30);
  assert.equal(delivered.length, 2);
  batcher.close();
});

test('per-entity updates are immediate and unmount cancels queued refreshes', async () => {
  const delivered = [];
  const immediate = createEngineEventBatcher(event => delivered.push(event), () => 0);
  immediate.push(change);
  immediate.push(change);
  assert.equal(delivered.length, 2);
  const queued = createEngineEventBatcher(event => delivered.push(event), () => 10);
  queued.push(change);
  queued.close();
  queued.push(change);
  await sleep(30);
  assert.equal(delivered.length, 2);
  immediate.close();
});
