import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { TaskService } from '../TaskService.js';
import { createMockLogger } from './helpers/mockLogger.js';
import { createMockFetch, mockFetchResponse, setupGlobalFetchMock } from './helpers/mockFetch.js';

describe('TaskService', () => {
  const mockLogger = createMockLogger();
  let fetchMock: ReturnType<typeof vi.fn>;
  let taskService: TaskService;

  beforeEach(() => {
    fetchMock = setupGlobalFetchMock();
    taskService = new TaskService(
      'http://localhost:8080',
      { id: 'task-1' } as any,
      'worker-1',
      mockLogger
    );
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe('complete()', () => {
    it('Sendet korrekten POST-Body an /api/service-task/:id/complete', async () => {
      mockFetchResponse(fetchMock, { ok: true, status: 204 });

      await taskService.complete();

      expect(fetchMock).toHaveBeenCalledWith(
        'http://localhost:8080/api/service-task/task-1/complete',
        expect.objectContaining({
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ workerId: 'worker-1', variables: undefined })
        })
      );
    });

    it('Ohne Variablen -> Body enthält variables: undefined', async () => {
      mockFetchResponse(fetchMock, { ok: true, status: 204 });
      await taskService.complete();
      const callArgs = fetchMock.mock.calls[0];
      const body = JSON.parse(callArgs[1].body);
      expect(body.variables).toBeUndefined();
    });

    it('Mit Variablen -> Body enthält die Variablen', async () => {
      mockFetchResponse(fetchMock, { ok: true, status: 204 });
      await taskService.complete({ test: 123 });
      const callArgs = fetchMock.mock.calls[0];
      const body = JSON.parse(callArgs[1].body);
      expect(body.variables).toEqual({ test: 123 });
    });

    it('Server antwortet 204 -> kein Error', async () => {
      mockFetchResponse(fetchMock, { ok: true, status: 204 });
      await expect(taskService.complete()).resolves.toBeUndefined();
    });

    it('Server antwortet 500 -> wirft Error mit Status und Body', async () => {
      mockFetchResponse(fetchMock, { ok: false, status: 500, text: 'Internal Error' });
      await expect(taskService.complete()).rejects.toThrow('Failed to complete task task-1: 500 Internal Error');
    });

    it('Server antwortet 404 -> wirft Error', async () => {
      mockFetchResponse(fetchMock, { ok: false, status: 404, text: 'Not Found' });
      await expect(taskService.complete()).rejects.toThrow('Failed to complete task task-1: 404 Not Found');
    });

    it('Logger-Integration: complete() loggt auf info-Level', async () => {
      mockFetchResponse(fetchMock, { ok: true, status: 204 });
      await taskService.complete();
      expect(mockLogger.info).toHaveBeenCalledWith('Task task-1 completed successfully');
    });
  });

  describe('failure()', () => {
    it('Sendet korrekten POST-Body an /api/service-task/:id/failure', async () => {
      mockFetchResponse(fetchMock, { ok: true, status: 204 });
      await taskService.failure('Test Error');

      expect(fetchMock).toHaveBeenCalledWith(
        'http://localhost:8080/api/service-task/task-1/failure',
        expect.objectContaining({
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
        })
      );
    });

    it('Default retries=0 wenn nicht angegeben', async () => {
      mockFetchResponse(fetchMock, { ok: true, status: 204 });
      await taskService.failure('Error');
      const body = JSON.parse(fetchMock.mock.calls[0][1].body);
      expect(body.retries).toBe(0);
      expect(body.errorMessage).toBe('Error');
      expect(body.errorDetails).toBeUndefined();
    });

    it('Mit retries=3, errorMessage und errorDetails', async () => {
      mockFetchResponse(fetchMock, { ok: true, status: 204 });
      await taskService.failure('Msg', 'Details', 3);
      const body = JSON.parse(fetchMock.mock.calls[0][1].body);
      expect(body.retries).toBe(3);
      expect(body.errorMessage).toBe('Msg');
      expect(body.errorDetails).toBe('Details');
    });

    it('Server-Fehler -> wirft Error', async () => {
      mockFetchResponse(fetchMock, { ok: false, status: 500 });
      await expect(taskService.failure('Err')).rejects.toThrow();
    });

    it('Logger-Integration: failure() loggt auf warn-Level', async () => {
      mockFetchResponse(fetchMock, { ok: true, status: 204 });
      await taskService.failure('Err');
      expect(mockLogger.warn).toHaveBeenCalledWith('Task task-1 reported as failed (retries=0): Err');
    });
  });

  describe('extendLock()', () => {
    it('Konvertiert ms korrekt in Sekunden (Math.ceil)', async () => {
      mockFetchResponse(fetchMock, { ok: true, status: 204 });
      await taskService.extendLock(1500); // 1.5s -> 2s
      const body = JSON.parse(fetchMock.mock.calls[0][1].body);
      expect(body.newDuration).toBe(2);
    });

    it('Sendet newDuration in Sekunden', async () => {
      mockFetchResponse(fetchMock, { ok: true, status: 204 });
      await taskService.extendLock(30000); // 30s
      const body = JSON.parse(fetchMock.mock.calls[0][1].body);
      expect(body.newDuration).toBe(30);
    });

    it('1500ms -> 2s (ceil)', async () => {
      mockFetchResponse(fetchMock, { ok: true, status: 204 });
      await taskService.extendLock(1500);
      const body = JSON.parse(fetchMock.mock.calls[0][1].body);
      expect(body.newDuration).toBe(2);
    });

    it('Server-Fehler -> wirft Error', async () => {
      mockFetchResponse(fetchMock, { ok: false, status: 500 });
      await expect(taskService.extendLock(1000)).rejects.toThrow();
    });
  });

  describe('bpmnError()', () => {
    it('Sendet errorCode an korrekte URL', async () => {
      mockFetchResponse(fetchMock, { ok: true, status: 204 });
      await taskService.bpmnError('ERR_CODE');
      
      expect(fetchMock).toHaveBeenCalledWith(
        'http://localhost:8080/api/service-task/task-1/bpmnError',
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify({ workerId: 'worker-1', errorCode: 'ERR_CODE' })
        })
      );
    });

    it('Server antwortet 204 -> kein Error', async () => {
      mockFetchResponse(fetchMock, { ok: true, status: 204 });
      await expect(taskService.bpmnError('Code')).resolves.toBeUndefined();
    });

    it('Server-Fehler -> wirft Error', async () => {
      mockFetchResponse(fetchMock, { ok: false, status: 400 });
      await expect(taskService.bpmnError('Code')).rejects.toThrow();
    });

    it('Logger-Integration: bpmnError() loggt auf info-Level', async () => {
      mockFetchResponse(fetchMock, { ok: true, status: 204 });
      await taskService.bpmnError('ERR_CODE');
      expect(mockLogger.info).toHaveBeenCalledWith("Task task-1: BPMN error 'ERR_CODE' thrown");
    });
  });
});
