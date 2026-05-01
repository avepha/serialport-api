import { Page, Route, expect } from '@playwright/test';

type JsonValue = null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue };
export type JsonObject = { [key: string]: JsonValue };

export type PortFixture = {
  name: string;
  type?: string;
  port_type?: string;
  manufacturer?: string | null;
  serial_number?: string | null;
};

export type ConnectionFixture = {
  name: string;
  status: string;
  port: string;
  baudRate: number;
  delimiter: string;
};

export type PresetFixture = {
  id: number;
  name: string;
  payload: JsonObject;
};

export type CapturedRequest = {
  method: string;
  path: string;
  body: unknown;
};

type MockState = {
  health: JsonObject;
  status: JsonObject;
  ports: PortFixture[];
  connections: ConnectionFixture[];
  presets: PresetFixture[];
  failures: Record<string, number>;
  requests: CapturedRequest[];
  nextPresetId: number;
  commandPostCount: number;
  presetDeleteCount: number;
};

type MockOptions = Partial<Pick<MockState, 'health' | 'status' | 'ports' | 'connections' | 'presets' | 'failures'>> & {
  event?: { name: string; data: JsonObject | string };
};

export const commandPayload = { method: 'query', topic: 'sensor.read', data: {} } satisfies JsonObject;

export const healthFixture = { status: 'ok', version: '0.1.0' } satisfies JsonObject;

export const statusFixture = {
  server: { status: 'ok', version: '0.1.0' },
  runtime: {
    mode: 'memory',
    realSerial: false,
    mockDevice: false,
    mockScriptConfigured: false,
  },
  serialDefaults: {
    defaultPortConfigured: false,
    baudRate: 115200,
    delimiter: '\r\n',
  },
  storage: {
    presets: 'memory',
    persistentPresets: false,
  },
} satisfies JsonObject;

export const portFixture = {
  name: '/dev/ttyUSB0',
  type: 'usb',
  manufacturer: 'Test Adapter',
  serial_number: 'TEST123',
} satisfies PortFixture;

export const connectionFixture = {
  name: 'default',
  status: 'connected',
  port: '/dev/ROBOT',
  baudRate: 115200,
  delimiter: '\r\n',
} satisfies ConnectionFixture;

export const presetFixture = {
  id: 1,
  name: 'Read sensor',
  payload: commandPayload,
} satisfies PresetFixture;

export async function installDashboardApiMocks(page: Page, options: MockOptions = {}) {
  const state: MockState = {
    health: options.health ?? healthFixture,
    status: options.status ?? statusFixture,
    ports: options.ports ?? [],
    connections: options.connections ? [...options.connections] : [],
    presets: options.presets ? [...options.presets] : [],
    failures: options.failures ?? {},
    requests: [],
    nextPresetId: nextPresetId(options.presets ?? []),
    commandPostCount: 0,
    presetDeleteCount: 0,
  };

  await installMockEventSource(page, options.event);

  await page.route('**/api/v1/**', async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    const path = url.pathname;
    const method = request.method();
    const body = request.postData() ? request.postDataJSON() : undefined;
    state.requests.push({ method, path, body });

    if (state.failures[path]) {
      return route.fulfill({
        status: state.failures[path],
        contentType: 'application/json',
        body: JSON.stringify({ error: 'simulated failure' }),
      });
    }

    if (method === 'GET' && path === '/api/v1/health') return json(route, state.health);
    if (method === 'GET' && path === '/api/v1/status') return json(route, state.status);
    if (method === 'GET' && path === '/api/v1/ports') return json(route, { ports: state.ports });
    if (method === 'GET' && path === '/api/v1/connections') return json(route, { connections: state.connections });
    if (method === 'GET' && path === '/api/v1/presets') return json(route, { presets: state.presets });

    if (method === 'POST' && path === '/api/v1/connections') {
      const connection = { ...(body as ConnectionFixture), status: 'connected' };
      state.connections = [connection];
      return json(route, { status: 'connected', connection });
    }

    const commandMatch = path.match(/^\/api\/v1\/connections\/([^/]+)\/commands$/);
    if (method === 'POST' && commandMatch) {
      state.commandPostCount += 1;
      const commandBody = body as { waitForResponse?: boolean };
      return json(
        route,
        commandBody.waitForResponse
          ? { status: 'ok', reqId: '1', response: { ok: true } }
          : { status: 'queued', reqId: '1' },
      );
    }

    if (method === 'POST' && path === '/api/v1/presets') {
      const requestBody = body as { name: string; payload: JsonObject };
      const preset = { id: state.nextPresetId++, name: requestBody.name, payload: requestBody.payload };
      state.presets = [preset];
      return json(route, { preset }, 201);
    }

    const presetDeleteMatch = path.match(/^\/api\/v1\/presets\/(\d+)$/);
    if (method === 'DELETE' && presetDeleteMatch) {
      const id = Number(presetDeleteMatch[1]);
      state.presetDeleteCount += 1;
      state.presets = state.presets.filter((preset) => preset.id !== id);
      return json(route, { status: 'deleted', id });
    }

    const connectionDeleteMatch = path.match(/^\/api\/v1\/connections\/([^/]+)$/);
    if (method === 'DELETE' && connectionDeleteMatch) {
      const name = decodeURIComponent(connectionDeleteMatch[1]);
      state.connections = state.connections.filter((connection) => connection.name !== name);
      return json(route, { status: 'disconnected', name });
    }

    return route.fulfill({ status: 404, contentType: 'application/json', body: JSON.stringify({ error: `unmocked ${method} ${path}` }) });
  });

  return state;
}

export function latestRequest(state: { requests: CapturedRequest[] }, method: string, path: string) {
  return [...state.requests].reverse().find((request) => request.method === method && request.path === path);
}

export function commandPostCount(state: { commandPostCount: number }) {
  return state.commandPostCount;
}

export function presetDeleteCount(state: { presetDeleteCount: number }) {
  return state.presetDeleteCount;
}

export async function expectRequestBody(state: { requests: CapturedRequest[] }, method: string, path: string, expectedBody: unknown) {
  await expect.poll(() => latestRequest(state, method, path)?.body).toEqual(expectedBody);
}

async function json(route: Route, body: unknown, status = 200) {
  return route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(body) });
}

function nextPresetId(presets: PresetFixture[]) {
  return presets.reduce((current, preset) => Math.max(current, preset.id + 1), 1);
}

async function installMockEventSource(page: Page, event?: { name: string; data: JsonObject | string }) {
  const mockEvent = event
    ? { name: event.name, data: typeof event.data === 'string' ? event.data : JSON.stringify(event.data) }
    : null;

  await page.addInitScript({
    content: `(() => {
      const mockEvent = ${JSON.stringify(mockEvent)};
      class MockEventSource {
        static CONNECTING = 0;
        static OPEN = 1;
        static CLOSED = 2;

        CONNECTING = 0;
        OPEN = 1;
        CLOSED = 2;
        readyState = MockEventSource.CONNECTING;
        onopen = null;
        onerror = null;
        onmessage = null;
        withCredentials = false;
        listeners = new Map();

        constructor(url) {
          this.url = String(url);
          window.setTimeout(() => {
            this.readyState = MockEventSource.OPEN;
            this.onopen?.(new Event('open'));
            if (mockEvent) this.dispatch(mockEvent.name, mockEvent.data);
          }, 0);
        }

        addEventListener(name, handler) {
          const handlers = this.listeners.get(name) ?? [];
          handlers.push(handler);
          this.listeners.set(name, handlers);
        }

        removeEventListener(name, handler) {
          this.listeners.set(name, (this.listeners.get(name) ?? []).filter((candidate) => candidate !== handler));
        }

        dispatchEvent(event) {
          this.dispatch(event.type, event.data);
          return true;
        }

        close() {
          this.readyState = MockEventSource.CLOSED;
        }

        dispatch(name, data) {
          const event = new MessageEvent(name, { data });
          for (const handler of this.listeners.get(name) ?? []) handler(event);
        }
      }
      window.EventSource = MockEventSource;
    })();`,
  });
}
