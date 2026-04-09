import { vi } from 'vitest';
import type { Logger } from '../../types.js';

export const createMockLogger = (): Logger => {
  const childMock = vi.fn().mockImplementation(() => createMockLogger());
  return {
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
    child: childMock,
  };
};
