export type HealthResponse = { status: string; version: string };
export type DashboardStatusResponse = {
  server: { status: "ok" | string; version: string };
  runtime: {
    mode: "memory" | "mock" | "mock-script" | "real";
    realSerial: boolean;
    mockDevice: boolean;
    mockScriptConfigured: boolean;
  };
  serialDefaults: {
    defaultPortConfigured: boolean;
    baudRate: number;
    delimiter: string;
  };
  storage: {
    presets: "memory" | "sqlite";
    persistentPresets: boolean;
  };
};
export type PortInfo = { name: string; type?: string; port_type?: string; manufacturer?: string | null; serial_number?: string | null };
export type PortsResponse = { ports: PortInfo[] };
export type ConnectionInfo = { name: string; status: string; port: string; baudRate: number; delimiter: string };
export type ConnectionsResponse = { connections: ConnectionInfo[] };
export type ConnectRequest = { name: string; port: string; baudRate: number; delimiter: string };
export type ConnectResponse = { status: string; connection: ConnectionInfo };
export type CommandRequest = { payload: Record<string, unknown>; waitForResponse?: boolean; timeoutMs?: number };
export type CommandResponse = { status: string; reqId: string; response?: unknown; error?: string };
export type Preset = { id: number; name: string; payload: Record<string, unknown> };
export type PresetsResponse = { presets: Preset[] };
export type PresetResponse = { preset: Preset };
export type DeletePresetResponse = { status: string; id: number };

async function requestJson<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, {
    ...init,
    headers: {
      ...(init?.body ? { "content-type": "application/json" } : {}),
      ...init?.headers,
    },
  });
  const text = await response.text();
  const data = text ? JSON.parse(text) : undefined;
  if (!response.ok) {
    const detail = data?.error ? `: ${data.error}` : "";
    throw new Error(`${response.status} ${response.statusText}${detail}`);
  }
  return data as T;
}

export const api = {
  health: () => requestJson<HealthResponse>("/api/v1/health"),
  status: () => requestJson<DashboardStatusResponse>("/api/v1/status"),
  ports: () => requestJson<PortsResponse>("/api/v1/ports"),
  connections: () => requestJson<ConnectionsResponse>("/api/v1/connections"),
  connect: (body: ConnectRequest) => requestJson<ConnectResponse>("/api/v1/connections", { method: "POST", body: JSON.stringify(body) }),
  disconnect: (name: string) => requestJson<{ status: string; name: string }>(`/api/v1/connections/${encodeURIComponent(name)}`, { method: "DELETE" }),
  sendCommand: (name: string, body: CommandRequest) => requestJson<CommandResponse>(`/api/v1/connections/${encodeURIComponent(name)}/commands`, { method: "POST", body: JSON.stringify(body) }),
  presets: () => requestJson<PresetsResponse>("/api/v1/presets"),
  createPreset: (name: string, payload: Record<string, unknown>) => requestJson<PresetResponse>("/api/v1/presets", { method: "POST", body: JSON.stringify({ name, payload }) }),
  deletePreset: (id: number) => requestJson<DeletePresetResponse>(`/api/v1/presets/${id}`, { method: "DELETE" }),
};

export function parseJsonObject(source: string): Record<string, unknown> {
  const parsed = JSON.parse(source) as unknown;
  if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
    throw new Error("Payload must be a JSON object");
  }
  return parsed as Record<string, unknown>;
}
