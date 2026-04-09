import type { ExternalTask } from '../../types.js';

export const createMockTask = (overrides?: Partial<ExternalTask>): ExternalTask => ({
  id: 'task-uuid-1234',
  instance_id: 'instance-uuid-5678',
  definition_key: 'process_payment',
  node_id: 'ServiceTask_1',
  topic: 'payment-topic',
  token_id: 'token-uuid-9012',
  variables_snapshot: {
    amount: 100,
    currency: 'USD'
  },
  created_at: new Date('2026-01-01T10:00:00Z').toISOString(),
  worker_id: null,
  lock_expiration: null,
  retries: 3,
  error_message: null,
  error_details: null,
  ...overrides,
});
