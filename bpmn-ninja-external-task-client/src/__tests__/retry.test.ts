import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { sleep, calculateBackoff, withRetry } from '../utils/retry.js';
import { createMockLogger } from './helpers/mockLogger.js';

describe('retry utility', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.clearAllMocks();
  });

  describe('sleep', () => {
    it('Wartet mindestens die angegebene Zeit', async () => {
      const sleepPromise = sleep(1000);
      vi.advanceTimersByTime(1000);
      await expect(sleepPromise).resolves.toBeUndefined();
    });
  });

  describe('calculateBackoff', () => {
    it('Korrekte Berechnung für Attempt 1, 2, 3, 4, 5', () => {
      const baseDelay = 1000;
      expect(calculateBackoff(1, baseDelay)).toBe(1000); // 1000 * 2^0
      expect(calculateBackoff(2, baseDelay)).toBe(2000); // 1000 * 2^1
      expect(calculateBackoff(3, baseDelay)).toBe(4000); // 1000 * 2^2
      expect(calculateBackoff(4, baseDelay)).toBe(8000); // 1000 * 2^3
      expect(calculateBackoff(5, baseDelay)).toBe(16000); // 1000 * 2^4
    });

    it('30s-Cap bei extrem hohen Attempts', () => {
      const baseDelay = 1000;
      expect(calculateBackoff(6, baseDelay)).toBe(30000); // would be 32000
      expect(calculateBackoff(10, baseDelay)).toBe(30000);
      expect(calculateBackoff(20, baseDelay)).toBe(30000);
    });
  });

  describe('withRetry', () => {
    const logger = createMockLogger();

    it('Sofortiger Erfolg beim ersten Versuch', async () => {
      const fn = vi.fn().mockResolvedValue('success');
      const result = await withRetry(fn, { maxRetries: 3, baseDelay: 1000, logger, label: 'test' });
      
      expect(result).toEqual({ success: true });
      expect(fn).toHaveBeenCalledTimes(1);
    });

    it('Erfolg nach 2 Fehlschlägen (3. Versuch erfolgreich)', async () => {
      const fn = vi.fn()
        .mockRejectedValueOnce(new Error('fail 1'))
        .mockRejectedValueOnce(new Error('fail 2'))
        .mockResolvedValueOnce('success');

      const promise = withRetry(fn, { maxRetries: 3, baseDelay: 1000, logger, label: 'test' });
      
      // Advance past the first delay (1000ms)
      await vi.advanceTimersByTimeAsync(1000);
      // Advance past the second delay (2000ms)
      await vi.advanceTimersByTimeAsync(2000);

      const result = await promise;
      expect(result).toEqual({ success: true });
      expect(fn).toHaveBeenCalledTimes(3);
      expect(logger.warn).toHaveBeenCalledTimes(2);
    });

    it('Alle Retries erschöpft -> gibt { success: false } zurück', async () => {
      const error = new Error('fatal error');
      const fn = vi.fn().mockRejectedValue(error);

      // maxRetries: 2 bedeutet 3 Versuche insgesamt (initial + 2 retries)
      const promise = withRetry(fn, { maxRetries: 2, baseDelay: 1000, logger, label: 'test' });
      
      await vi.advanceTimersByTimeAsync(1000); // delay 1
      await vi.advanceTimersByTimeAsync(2000); // delay 2
      
      const result = await promise;
      expect(result).toEqual({ success: false, error, attempts: 3 });
      expect(fn).toHaveBeenCalledTimes(3);
    });

    it('Korrekte Anzahl an Versuchen (maxRetries + 1)', async () => {
      const fn = vi.fn().mockRejectedValue(new Error('fail'));
      const promise = withRetry(fn, { maxRetries: 4, baseDelay: 10, logger, label: 'test' });
      await vi.advanceTimersByTimeAsync(10000); // advance enough time to flush all timers
      await promise;
      
      expect(fn).toHaveBeenCalledTimes(5);
    });

    it('Error-Objekt wird korrekt durchgereicht', async () => {
      const customError = new Error('Custom Error');
      customError.name = 'HttpError';
      const fn = vi.fn().mockRejectedValue(customError);

      const promise = withRetry(fn, { maxRetries: 1, baseDelay: 10, logger, label: 'test' });
      await vi.advanceTimersByTimeAsync(100);
      const result = await promise;
      
      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error).toBe(customError);
      }
    });

    it('Wartezeiten zwischen Retries steigen exponentiell (mock sleep und prüfe Aufrufe)', async () => {
      const fn = vi.fn().mockRejectedValue(new Error('fail'));
      
      const promise = withRetry(fn, { maxRetries: 3, baseDelay: 1000, logger, label: 'test' });
      
      await vi.advanceTimersByTimeAsync(1000); // Attempt 1 fails -> wait 1000
      expect(fn).toHaveBeenCalledTimes(2);

      await vi.advanceTimersByTimeAsync(2000); // Attempt 2 fails -> wait 2000
      expect(fn).toHaveBeenCalledTimes(3);

      await vi.advanceTimersByTimeAsync(4000); // Attempt 3 fails -> wait 4000
      expect(fn).toHaveBeenCalledTimes(4);

      await promise; 
    });

    it('Logger wird bei jedem Fehlversuch korrekt aufgerufen (warn für Retry, error für final)', async () => {
      const fn = vi.fn().mockRejectedValue(new Error('fail'));
      const promise = withRetry(fn, { maxRetries: 1, baseDelay: 1000, logger, label: 'test-label' });
      
      await vi.advanceTimersByTimeAsync(1000);
      await promise;

      expect(logger.warn).toHaveBeenCalledWith(
        expect.stringContaining('[test-label] Handler failed (attempt 1/2)')
      );

      expect(logger.error).toHaveBeenCalledWith(
        expect.stringContaining('[test-label] Handler failed after 2 attempts')
      );
    });

    it('maxRetries=0 -> nur ein Versuch, kein Retry', async () => {
      const fn = vi.fn().mockRejectedValue(new Error('fail'));
      const promise = withRetry(fn, { maxRetries: 0, baseDelay: 1000, logger, label: 'test' });
      
      const result = await promise;
      expect(fn).toHaveBeenCalledTimes(1);
      expect(result.success).toBe(false);
    });

    it('Nicht-Error-Throws (z.B. String) werden in Error gewrapped', async () => {
      const fn = vi.fn().mockRejectedValue('String Error');
      const promise = withRetry(fn, { maxRetries: 0, baseDelay: 100, logger, label: 'test' });
      const result = await promise;
      
      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error).toBeInstanceOf(Error);
        expect(result.error!.message).toContain('String Error');
      }
    });

    it('Label erscheint in Log-Nachrichten', async () => {
      const fn = vi.fn().mockRejectedValue(new Error('fail'));
      const promise = withRetry(fn, { maxRetries: 1, baseDelay: 100, logger, label: 'MY_CUSTOM_LABEL' });
      
      await vi.advanceTimersByTimeAsync(1000);
      await promise;

      expect(logger.warn).toHaveBeenCalledWith(
        expect.stringContaining('[MY_CUSTOM_LABEL] Handler failed')
      );
    });
  });
});
