import { vi } from 'vitest';

export interface MockFetchResponse {
  ok: boolean;
  status: number;
  json?: any;
  text?: string;
}

export const createMockFetch = () => {
  return vi.fn().mockImplementation((_url: string | URL | Request, _init?: RequestInit) => {
    return Promise.resolve(new Response(null, { status: 404 }));
  });
};

export const mockFetchResponse = (fetchMock: ReturnType<typeof vi.fn>, response: MockFetchResponse) => {
  fetchMock.mockResolvedValueOnce({
    ...response,
    json: vi.fn().mockResolvedValue(response.json || {}),
    text: vi.fn().mockResolvedValue(response.text || ''),
  } as unknown as Response);
};

export const setupGlobalFetchMock = () => {
  const fetchMock = createMockFetch();
  vi.stubGlobal('fetch', fetchMock);
  return fetchMock;
};
