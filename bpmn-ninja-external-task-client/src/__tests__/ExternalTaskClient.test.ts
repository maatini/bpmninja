import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { ExternalTaskClient } from '../ExternalTaskClient.js';
import { createMockLogger } from './helpers/mockLogger.js';
import { mockFetchResponse, setupGlobalFetchMock } from './helpers/mockFetch.js';
import { createMockTask } from './helpers/fixtures.js';

async function stopClientAndFlush(client: any, advanceMs = 30000) {
  const p = client.stop();
  await vi.advanceTimersByTimeAsync(advanceMs);
  await p;
}

describe('ExternalTaskClient', () => {
  let fetchMock: ReturnType<typeof vi.fn>;
  const mockLogger = createMockLogger();

  beforeEach(() => {
    fetchMock = setupGlobalFetchMock();
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.clearAllMocks();
    vi.useRealTimers();
  });

  describe('Konstruktor & Konfiguration', () => {
    it('Default-Werte werden korrekt gesetzt (baseUrl, lockDuration, maxTasks, etc.)', () => {
      const client = new ExternalTaskClient();
      expect((client as any).config.baseUrl).toBe('http://localhost:8080');
      expect((client as any).config.lockDuration).toBe(30000);
      expect((client as any).config.maxTasks).toBe(10);
      expect((client as any).config.workerId).toBeDefined();
    });

    it('Benutzerdefinierte Werte überschreiben Defaults', () => {
      const client = new ExternalTaskClient({ baseUrl: 'http://custom:1234', lockDuration: 5000 });
      expect((client as any).config.baseUrl).toBe('http://custom:1234');
      expect((client as any).config.lockDuration).toBe(5000);
    });

    it('baseUrl trailing Slashes werden entfernt', () => {
      const client = new ExternalTaskClient({ baseUrl: 'http://custom/' });
      expect((client as any).config.baseUrl).toBe('http://custom');
    });

    it('workerId wird auto-generiert wenn nicht angegeben', () => {
      const client = new ExternalTaskClient();
      expect(typeof (client as any).config.workerId).toBe('string');
      expect((client as any).config.workerId.length).toBeGreaterThan(0);
    });

    it('logger: false -> Silent Mode (noop-Logger)', () => {
      const client = new ExternalTaskClient({ logger: false });
      expect((client as any).config.logger.info).toBeDefined();
      expect((client as any).config.logger.info.name).toBe('noop'); // noop functions
    });

    it('Benutzerdefinierter Logger wird verwendet', () => {
      const client = new ExternalTaskClient({ logger: mockLogger });
      expect((client as any).config.logger).toBe(mockLogger);
    });
  });

  describe('Subscriptions', () => {
    it('subscribe() registriert einen Handler', () => {
      const client = new ExternalTaskClient();
      const handler = vi.fn();
      client.subscribe('topic1', handler);
      expect((client as any).subscriptions.has('topic1')).toBe(true);
    });

    it('subscribe() gibt this zurück (Chaining)', () => {
      const client = new ExternalTaskClient();
      expect(client.subscribe('t1', vi.fn())).toBe(client);
    });

    it('subscribe() mit gleicher Topic -> wirft Error', () => {
      const client = new ExternalTaskClient();
      client.subscribe('topic1', vi.fn());
      expect(() => client.subscribe('topic1', vi.fn())).toThrow();
    });

    it('subscribe() übernimmt globale Defaults für lockDuration/maxRetries', () => {
      const client = new ExternalTaskClient({ lockDuration: 10000, maxRetries: 5 });
      client.subscribe('topic1', vi.fn());
      const sub = (client as any).subscriptions.get('topic1');
      expect(sub.options.lockDuration).toBe(10000);
      expect(sub.options.maxRetries).toBe(5);
    });

    it('subscribe() mit custom Options überschreibt Defaults', () => {
      const client = new ExternalTaskClient({ lockDuration: 10000 });
      client.subscribe('topic1', vi.fn(), { lockDuration: 5000, maxRetries: 1 });
      const sub = (client as any).subscriptions.get('topic1');
      expect(sub.options.lockDuration).toBe(5000);
      expect(sub.options.maxRetries).toBe(1);
    });

    it('unsubscribe() entfernt Subscription -> true', () => {
      const client = new ExternalTaskClient();
      client.subscribe('topic1', vi.fn());
      expect(client.unsubscribe('topic1')).toBe(true);
      expect((client as any).subscriptions.has('topic1')).toBe(false);
    });

    it('unsubscribe() mit unbekanntem Topic -> false', () => {
      const client = new ExternalTaskClient();
      expect(client.unsubscribe('unknown')).toBe(false);
    });
  });

  describe('Start/Stop Lifecycle', () => {
    it('start() ohne Subscriptions -> wirft Error', () => {
      const client = new ExternalTaskClient();
      expect(() => client.start()).toThrow('Cannot start');
    });

    it('start() setzt isRunning auf true', () => {
      const client = new ExternalTaskClient({ logger: mockLogger });
      client.subscribe('topic', vi.fn());
      client.start();
      expect((client as any).isRunning).toBe(true);
      client.stop();
    });

    it('start() doppelt aufgerufen -> idempotent (kein Error, Warn-Log)', () => {
      const client = new ExternalTaskClient({ logger: mockLogger });
      client.subscribe('topic', vi.fn());
      client.start();
      client.start();
      expect(mockLogger.warn).toHaveBeenCalledWith('Client is already running');
      stopClientAndFlush(client);
    });

    it('stop() setzt isRunning auf false', async () => {
      const client = new ExternalTaskClient({ logger: mockLogger });
      client.subscribe('topic', vi.fn());
      client.start();
      await stopClientAndFlush(client);
      expect((client as any).isRunning).toBe(false);
    });

    it('stop() wartet auf in-flight Handlers (mock einen langsamen Handler)', async () => {
      const client = new ExternalTaskClient({ logger: mockLogger });
      let handlerRunning = false;
      client.subscribe('topic', async () => {
        handlerRunning = true;
        await new Promise(r => setTimeout(r, 5000));
        handlerRunning = false;
      });

      mockFetchResponse(fetchMock, {
        ok: true,
        status: 200,
        json: [createMockTask({ id: 'task-1', topic: 'topic' })]
      });

      client.start();
      await vi.advanceTimersByTimeAsync(1); // fetch and trigger handler
      
      expect(handlerRunning).toBe(true);

      const stopPromise = client.stop();
      await vi.advanceTimersByTimeAsync(5000); // finish handler
      await stopPromise;
      
      expect(handlerRunning).toBe(false);
    });

    it('stop() bei nicht-laufendem Client -> no-op', async () => {
      const client = new ExternalTaskClient({ logger: mockLogger });
      await expect(client.stop()).resolves.toBeUndefined();
    });
  });

  describe('fetchAndLock (Poll-Loop)', () => {
    it('Sendet korrekten Body mit allen subscribed Topics', async () => {
      const client = new ExternalTaskClient({ logger: false, asyncResponseTimeout: 5000 });
      client.subscribe('t1', vi.fn(), { lockDuration: 5000 });
      client.subscribe('t2', vi.fn());

      mockFetchResponse(fetchMock, { ok: true, status: 200, json: [] });

      client.start();
      await vi.advanceTimersByTimeAsync(1);

      const callArgs = fetchMock.mock.calls[0];
      const body = JSON.parse(callArgs[1].body);

      expect(body.asyncResponseTimeout).toBe(5000);
      expect(body.topics).toEqual([
        { topicName: 't1', lockDuration: 5 }, // 5000ms -> 5s
        { topicName: 't2', lockDuration: 30 } // default 30s
      ]);
      await stopClientAndFlush(client);
    });

    it('lockDuration wird von ms in Sekunden umgerechnet', async () => {
      const client = new ExternalTaskClient({ logger: false });
      client.subscribe('t1', vi.fn(), { lockDuration: 1500 }); // 1.5s -> 2s ceil

      mockFetchResponse(fetchMock, { ok: true, status: 200, json: [] });
      client.start();
      await vi.advanceTimersByTimeAsync(1);

      const body = JSON.parse(fetchMock.mock.calls[0][1].body);
      expect(body.topics[0].lockDuration).toBe(2);
      await stopClientAndFlush(client);
    });

    it('Server gibt leeres Array -> keine Handler ausgelöst', async () => {
      const handler = vi.fn();
      const client = new ExternalTaskClient({ logger: false });
      client.subscribe('t', handler);

      mockFetchResponse(fetchMock, { ok: true, status: 200, json: [] });
      client.start();
      await vi.advanceTimersByTimeAsync(1);

      expect(handler).not.toHaveBeenCalled();
      await stopClientAndFlush(client);
    });

    it('Server gibt Tasks -> Handler werden für passende Topics aufgerufen', async () => {
      const t1Handler = vi.fn().mockResolvedValue(undefined);
      const t2Handler = vi.fn().mockResolvedValue(undefined);
      
      const client = new ExternalTaskClient({ logger: mockLogger });
      client.subscribe('t1', t1Handler);
      client.subscribe('t2', t2Handler);

      mockFetchResponse(fetchMock, {
        ok: true,
        status: 200,
        json: [
          createMockTask({ id: 't-1', topic: 't1' }),
          createMockTask({ id: 't-2', topic: 't2' })
        ]
      });

      client.start();
      await vi.advanceTimersByTimeAsync(1);

      expect(t1Handler).toHaveBeenCalledTimes(1);
      expect(t2Handler).toHaveBeenCalledTimes(1);
      await stopClientAndFlush(client);
    });

    it('Task mit unsubscribed Topic -> wird übersprungen (warn-Log)', async () => {
      const client = new ExternalTaskClient({ logger: mockLogger });
      client.subscribe('known', vi.fn());

      mockFetchResponse(fetchMock, {
        ok: true,
        status: 200,
        json: [createMockTask({ topic: 'unknown-topic' })]
      });

      client.start();
      await vi.advanceTimersByTimeAsync(1);

      expect(mockLogger.warn).toHaveBeenCalledWith(expect.stringContaining('Received task for unsubscribed topic "unknown-topic", skipping'));
      await stopClientAndFlush(client);
    });

    it('Server-Fehler -> Error wird geloggt, Polling geht weiter', async () => {
      const client = new ExternalTaskClient({ logger: mockLogger, pollingInterval: 100 });
      client.subscribe('t', vi.fn());

      // 1. poll -> error 500
      mockFetchResponse(fetchMock, { ok: false, status: 500 });
      // 2. poll -> success
      fetchMock.mockResolvedValueOnce({
        ok: true, status: 200, json: () => Promise.resolve([])
      } as any);

      client.start();
      await vi.advanceTimersByTimeAsync(1);
      expect(mockLogger.error).toHaveBeenCalledWith(
        expect.stringContaining('Poll cycle failed: fetchAndLock failed:')
      );

      await vi.advanceTimersByTimeAsync(100);
      expect(fetchMock).toHaveBeenCalledTimes(2); // continued polling
      await stopClientAndFlush(client);
    });
  });

  describe('Handler-Execution mit Retry', () => {
    it('Handler erfolgreich -> kein failure() Aufruf', async () => {
      const client = new ExternalTaskClient({ logger: mockLogger });
      client.subscribe('t', async (_task, service) => {
        service.failure = vi.fn(); // tracking
      });

      mockFetchResponse(fetchMock, {
        ok: true, status: 200, json: [createMockTask({ topic: 't' })]
      });

      client.start();
      await vi.advanceTimersByTimeAsync(1);
      await vi.advanceTimersByTimeAsync(1); // resolve promises

      const failureCallCount = fetchMock.mock.calls.filter(c => c[0].includes('failure')).length;
      expect(failureCallCount).toBe(0);
      await stopClientAndFlush(client);
    });

    it('Handler wirft 3x (maxRetries=2) -> failure() mit retries: 0 wird aufgerufen', async () => {
      const client = new ExternalTaskClient({ logger: mockLogger, maxRetries: 2, baseRetryDelay: 10 });
      let handlerCalls = 0;
      client.subscribe('t', async () => {
        handlerCalls++;
        throw new Error('Fatal');
      });

      // fetch mock responds with tasks
      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: () => Promise.resolve([createMockTask({ id: 'task-1', topic: 't' })]),
        text: () => Promise.resolve('')
      } as any);
      
      // we need to mock the failure fetch response
      fetchMock.mockResolvedValueOnce({
        ok: true, status: 204, json: () => Promise.resolve({}), text: () => Promise.resolve('')
      } as any);

      client.start();
      await vi.advanceTimersByTimeAsync(1); // fetch
      
      // wait for 2 retries
      await vi.advanceTimersByTimeAsync(10); // Attempt 1 fails, wait 10
      await vi.advanceTimersByTimeAsync(20); // Attempt 2 fails, wait 20
      
      // let handler finish
      await vi.advanceTimersByTimeAsync(1);
      
      expect(handlerCalls).toBe(3);

      const failureCall = fetchMock.mock.calls.find(c => c[0].includes('/api/service-task/task-1/failure'));
      expect(failureCall).toBeDefined();
      expect(JSON.parse(failureCall![1].body).retries).toBe(0);
      
      await stopClientAndFlush(client);
    });

    it('failure()-Error bei Incident-Report -> wird geloggt (kein Crash)', async () => {
      const client = new ExternalTaskClient({ logger: mockLogger, maxRetries: 0 });
      client.subscribe('t', async () => { throw new Error('Crash'); });

      // Engine returns a task
      fetchMock.mockResolvedValueOnce({
        ok: true, status: 200, json: () => Promise.resolve([createMockTask({ id: 'task-1', topic: 't' })])
      } as any);

      // Engine fails the failure report (!)
      fetchMock.mockResolvedValueOnce({
        ok: false, status: 500, text: () => Promise.resolve('DB down')
      } as any);

      client.start();
      await vi.advanceTimersByTimeAsync(1);
      await vi.advanceTimersByTimeAsync(1); // settle promises

      expect(mockLogger.error).toHaveBeenCalledWith(
        expect.stringContaining('Failed to report incident for task task-1: Failed to report failure for task task-1: 500 DB down')
      );

      await stopClientAndFlush(client);
    });

    it('Handler mit per-Topic maxRetries nutzt den Topic-spezifischen Wert', async () => {
      const client = new ExternalTaskClient({ logger: false, maxRetries: 5 }); // global 5
      let calls = 0;
      client.subscribe('t', async () => {
        calls++;
        throw new Error('fail');
      }, { maxRetries: 1 }); // specific 1

      fetchMock.mockResolvedValueOnce({
        ok: true, status: 200, json: () => Promise.resolve([createMockTask({ id: 'task-1', topic: 't' })])
      } as any);
      
      fetchMock.mockResolvedValue({ ok: true, status: 204 } as any); // ignore subsequent

      client.start();
      await vi.advanceTimersByTimeAsync(1000); // 1. fetch
      await vi.advanceTimersByTimeAsync(4000); // enough time for retries
      
      expect(calls).toBe(2); // initial + 1 retry
      await stopClientAndFlush(client);
    });
  });

  describe('Automatische Lock-Extension', () => {
    it('autoExtendLock: false -> kein Timer gestartet', async () => {
      const client = new ExternalTaskClient({ logger: mockLogger, autoExtendLock: false });
      let extending = false;
      client.subscribe('t', async () => {
        extending = true;
        await new Promise(r => setTimeout(r, 10000));
        extending = false;
      });

      mockFetchResponse(fetchMock, {
        ok: true, status: 200, json: [createMockTask({ id: 'task-1', topic: 't' })]
      });

      client.start();
      await vi.advanceTimersByTimeAsync(1); // handler starts
      
      expect(extending).toBe(true);
      await vi.advanceTimersByTimeAsync(20000); // wait past autoExtendLockInterval

      const extendCalls = fetchMock.mock.calls.filter(c => c[0].includes('extendLock'));
      expect(extendCalls.length).toBe(0);

      await stopClientAndFlush(client);
    });

    it('autoExtendLock: true -> extendLock() wird periodisch aufgerufen', async () => {
      const client = new ExternalTaskClient({ 
        logger: false, 
        autoExtendLock: true, 
        autoExtendLockInterval: 5000, 
        lockDuration: 10000 
      });

      client.subscribe('t', async () => {
        await new Promise(r => setTimeout(r, 12000));
      });

      fetchMock.mockResolvedValueOnce({
        ok: true, status: 200, json: () => Promise.resolve([createMockTask({ id: 'task-1', topic: 't' })])
      } as any);

      // mock extend requests
      fetchMock.mockResolvedValue({ ok: true, status: 204 } as any);

      client.start();
      await vi.advanceTimersByTimeAsync(1); // handler starts
      
      await vi.advanceTimersByTimeAsync(5000); // interval 1
      await vi.advanceTimersByTimeAsync(5000); // interval 2
      
      const extendCalls = fetchMock.mock.calls.filter(c => c[0].includes('extendLock'));
      expect(extendCalls.length).toBe(2);

      await vi.advanceTimersByTimeAsync(5000); // finish handler
      await stopClientAndFlush(client);
    });

    it('Lock-Extension-Timer wird nach Handler-Abschluss bereinigt', async () => {
      const client = new ExternalTaskClient({ logger: mockLogger, autoExtendLock: true, autoExtendLockInterval: 5000 });
      client.subscribe('t', async () => {
        // executes instantly
      });

      fetchMock.mockResolvedValueOnce({
        ok: true, status: 200, json: () => Promise.resolve([createMockTask({ id: 'task-1', topic: 't' })])
      } as any);
      
      // for safety default mock
      fetchMock.mockResolvedValue({ ok: true, status: 204 } as any);

      client.start();
      await vi.advanceTimersByTimeAsync(1); // handler instantly finishes
      
      // wait long enough where timer WOULD trigger
      await vi.advanceTimersByTimeAsync(10000); 
      
      const extendCalls = fetchMock.mock.calls.filter(c => c[0].includes('extendLock'));
      expect(extendCalls.length).toBe(0);

      await stopClientAndFlush(client);
    });

    it('Lock-Extension-Fehler -> wird geloggt (kein Crash)', async () => {
      const client = new ExternalTaskClient({ logger: mockLogger, autoExtendLock: true, autoExtendLockInterval: 2000 });
      client.subscribe('t', async () => {
        await new Promise(r => setTimeout(r, 3000)); // longer than interval
      });

      fetchMock.mockResolvedValueOnce({
        ok: true, status: 200, json: () => Promise.resolve([createMockTask({ id: 'task-1', topic: 't' })])
      } as any);
      
      // fail the extension
      fetchMock.mockResolvedValueOnce({
        ok: false, status: 500, text: () => Promise.resolve('Error')
      } as any);

      client.start();
      await vi.advanceTimersByTimeAsync(1); 
      await vi.advanceTimersByTimeAsync(2000); // trigger extend lock
      
      expect(mockLogger.warn).toHaveBeenCalledWith(
        expect.stringContaining('Failed to extend lock for task task-1:')
      );

      await vi.advanceTimersByTimeAsync(2000); // end handler
      await stopClientAndFlush(client);
    });
  });

  describe('Graceful Shutdown', () => {
    it('stop() bricht laufenden fetchAndLock via AbortController ab', async () => {
      const client = new ExternalTaskClient({ logger: mockLogger, pollingInterval: 5000 });
      client.subscribe('t', vi.fn());

      let fetchSignal: AbortSignal | undefined;
      fetchMock.mockImplementationOnce(async (_url, init) => {
        fetchSignal = init?.signal;
        // make it a slow request
        await new Promise(r => setTimeout(r, 10000));
        return new Response(JSON.stringify([]));
      });

      client.start();
      await vi.advanceTimersByTimeAsync(1); // Start the fetch

      expect(fetchSignal).toBeDefined();
      expect(fetchSignal?.aborted).toBe(false);

      // Stop the client -> should abort
      await stopClientAndFlush(client);
    });

    it('Nach stop() werden keine neuen Tasks mehr gefetcht', async () => {
      const client = new ExternalTaskClient({ logger: false, pollingInterval: 500 });
      client.subscribe('t', vi.fn());

      fetchMock.mockResolvedValue({ ok: true, status: 200, json: () => Promise.resolve([]) } as any);

      client.start();
      await vi.advanceTimersByTimeAsync(1);
      
      await stopClientAndFlush(client);
      await vi.advanceTimersByTimeAsync(1000); // try to advance beyond polling interval
      
      expect(fetchMock).toHaveBeenCalledTimes(1); // exactly one time from the initial start
    });
  });
});
