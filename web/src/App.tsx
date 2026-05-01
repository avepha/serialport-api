import { useEffect, useMemo, useState } from "react";
import { Activity, Cable, CircleDot, Play, PlugZap, RefreshCw, Save, Send, Server, Trash2 } from "lucide-react";
import { api, ConnectionInfo, HealthResponse, parseJsonObject, PortInfo, Preset } from "./api";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Separator } from "@/components/ui/separator";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Textarea } from "@/components/ui/textarea";

type Notice = { tone: "ok" | "error"; message: string } | null;
type EventLogItem = { id: number; event: string; data: string };

const defaultPayload = JSON.stringify({ method: "query", topic: "sensor.read", data: {} }, null, 2);

function App() {
  const [health, setHealth] = useState<HealthResponse | null>(null);
  const [ports, setPorts] = useState<PortInfo[]>([]);
  const [connections, setConnections] = useState<ConnectionInfo[]>([]);
  const [presets, setPresets] = useState<Preset[]>([]);
  const [selectedConnection, setSelectedConnection] = useState("");
  const [notice, setNotice] = useState<Notice>(null);
  const [loading, setLoading] = useState(false);
  const [events, setEvents] = useState<EventLogItem[]>([]);
  const [eventStatus, setEventStatus] = useState("connecting");

  const [connectForm, setConnectForm] = useState({ name: "default", port: "/dev/ROBOT", baudRate: "115200", delimiter: "\\r\\n" });
  const [payload, setPayload] = useState(defaultPayload);
  const [waitForResponse, setWaitForResponse] = useState(false);
  const [timeoutMs, setTimeoutMs] = useState("2000");
  const [presetName, setPresetName] = useState("Read sensor");

  const refresh = async () => {
    setLoading(true);
    try {
      const [healthResult, portsResult, connectionsResult, presetsResult] = await Promise.all([
        api.health(),
        api.ports(),
        api.connections(),
        api.presets(),
      ]);
      setHealth(healthResult);
      setPorts(portsResult.ports);
      setConnections(connectionsResult.connections);
      setPresets(presetsResult.presets);
      if (!selectedConnection && connectionsResult.connections[0]) {
        setSelectedConnection(connectionsResult.connections[0].name);
      }
      setNotice({ tone: "ok", message: "Dashboard data refreshed" });
    } catch (error) {
      setNotice({ tone: "error", message: errorMessage(error) });
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void refresh();
  }, []);

  useEffect(() => {
    const source = new EventSource("/api/v1/events");
    source.onopen = () => setEventStatus("connected");
    const append = (eventName: string, data: string) => {
      setEvents((current) => [{ id: Date.now() + Math.random(), event: eventName, data }, ...current].slice(0, 100));
    };
    ["serial.json", "serial.text", "serial.log", "serial.notification", "serial.error"].forEach((eventName) => {
      source.addEventListener(eventName, (event) => append(eventName, (event as MessageEvent).data));
    });
    source.onerror = () => setEventStatus(source.readyState === EventSource.CLOSED ? "closed" : "reconnecting to live stream");
    return () => source.close();
  }, []);

  const connectionOptions = useMemo(() => connections.map((connection) => connection.name), [connections]);

  const connect = async () => {
    try {
      const response = await api.connect({
        name: connectForm.name.trim(),
        port: connectForm.port.trim(),
        baudRate: Number(connectForm.baudRate),
        delimiter: decodeDelimiter(connectForm.delimiter),
      });
      setSelectedConnection(response.connection.name);
      setNotice({ tone: "ok", message: `Connected ${response.connection.name}` });
      await refresh();
    } catch (error) {
      setNotice({ tone: "error", message: errorMessage(error) });
    }
  };

  const disconnect = async (name: string) => {
    try {
      await api.disconnect(name);
      setNotice({ tone: "ok", message: `Disconnected ${name}` });
      await refresh();
    } catch (error) {
      setNotice({ tone: "error", message: errorMessage(error) });
    }
  };

  const sendCommand = async (overridePayload?: Record<string, unknown>) => {
    if (!selectedConnection) {
      setNotice({ tone: "error", message: "Select or create a connection first" });
      return;
    }
    try {
      const response = await api.sendCommand(selectedConnection, {
        payload: overridePayload ?? parseJsonObject(payload),
        waitForResponse,
        timeoutMs: waitForResponse ? Number(timeoutMs) : undefined,
      });
      setNotice({ tone: "ok", message: `Command ${response.reqId} ${response.status}` });
    } catch (error) {
      setNotice({ tone: "error", message: errorMessage(error) });
    }
  };

  const savePreset = async () => {
    try {
      await api.createPreset(presetName.trim(), parseJsonObject(payload));
      setNotice({ tone: "ok", message: `Saved preset ${presetName}` });
      await refresh();
    } catch (error) {
      setNotice({ tone: "error", message: errorMessage(error) });
    }
  };

  const removePreset = async (id: number) => {
    try {
      await api.deletePreset(id);
      setNotice({ tone: "ok", message: "Preset deleted" });
      await refresh();
    } catch (error) {
      setNotice({ tone: "error", message: errorMessage(error) });
    }
  };

  const applyPreset = (preset: Preset) => {
    setPayload(JSON.stringify(preset.payload, null, 2));
    setNotice({ tone: "ok", message: `Loaded preset ${preset.name}` });
  };

  return (
    <main className="min-h-screen bg-[radial-gradient(circle_at_top_left,rgba(14,165,233,0.18),transparent_36rem)] p-4 md:p-8">
      <div className="mx-auto flex max-w-7xl flex-col gap-5">
        <header className="flex flex-col gap-3 md:flex-row md:items-end md:justify-between">
          <div>
            <div className="flex items-center gap-2 text-sm text-sky-300"><PlugZap className="h-4 w-4" /> serialport-api</div>
            <h1 className="mt-2 text-3xl font-semibold tracking-tight">Serial control dashboard</h1>
            <p className="mt-2 max-w-2xl text-sm text-muted-foreground">Hardware-free by default, real serial when the server is started with --real-serial. All controls use existing API routes.</p>
          </div>
          <Button onClick={refresh} disabled={loading} variant="secondary"><RefreshCw className="mr-2 h-4 w-4" />Refresh</Button>
        </header>

        {notice && (
          <Alert className={notice.tone === "error" ? "border-destructive/60 bg-destructive/10" : "border-sky-500/40 bg-sky-500/10"}>
            <AlertTitle>{notice.tone === "error" ? "Action failed" : "Ready"}</AlertTitle>
            <AlertDescription>{notice.message}</AlertDescription>
          </Alert>
        )}

        <section className="grid gap-4 md:grid-cols-3">
          <StatusCard icon={<Server className="h-4 w-4" />} label="Server" value={health ? `${health.status} · v${health.version}` : "unknown"} />
          <StatusCard icon={<Cable className="h-4 w-4" />} label="Ports" value={`${ports.length} visible`} />
          <StatusCard icon={<Activity className="h-4 w-4" />} label="EventSource" value={eventStatus} />
        </section>

        <Tabs defaultValue="control" className="space-y-4">
          <TabsList><TabsTrigger value="control">Control</TabsTrigger><TabsTrigger value="events">Events</TabsTrigger><TabsTrigger value="presets">Presets</TabsTrigger></TabsList>
          <TabsContent value="control" className="grid gap-4 lg:grid-cols-[0.95fr_1.05fr]">
            <Card>
              <CardHeader><CardTitle>Connections</CardTitle><CardDescription>Create, inspect, and disconnect named serial sessions.</CardDescription></CardHeader>
              <CardContent className="space-y-4">
                <div className="grid gap-3 md:grid-cols-2">
                  <Field label="Name"><Input value={connectForm.name} onChange={(e) => setConnectForm({ ...connectForm, name: e.target.value })} /></Field>
                  <Field label="Port"><Input value={connectForm.port} onChange={(e) => setConnectForm({ ...connectForm, port: e.target.value })} /></Field>
                  <Field label="Baud"><Input inputMode="numeric" value={connectForm.baudRate} onChange={(e) => setConnectForm({ ...connectForm, baudRate: e.target.value })} /></Field>
                  <Field label="Delimiter"><Input value={connectForm.delimiter} onChange={(e) => setConnectForm({ ...connectForm, delimiter: e.target.value })} /></Field>
                </div>
                <Button onClick={connect}><CircleDot className="mr-2 h-4 w-4" />Connect</Button>
                <Separator />
                {connections.length === 0 ? <p className="text-sm text-muted-foreground">No active connections. Create one to enable commands.</p> : (
                  <Table><TableHeader><TableRow><TableHead>Name</TableHead><TableHead>Port</TableHead><TableHead>Status</TableHead><TableHead /></TableRow></TableHeader><TableBody>
                    {connections.map((connection) => <TableRow key={connection.name}><TableCell>{connection.name}</TableCell><TableCell>{connection.port}</TableCell><TableCell><Badge variant="secondary">{connection.status}</Badge></TableCell><TableCell><Button size="sm" variant="destructive" onClick={() => void disconnect(connection.name)}><Trash2 className="mr-1 h-3 w-3" />Drop</Button></TableCell></TableRow>)}
                  </TableBody></Table>
                )}
                <Field label="Command target">
                  <Select value={selectedConnection} onValueChange={setSelectedConnection} disabled={connectionOptions.length === 0}>
                    <SelectTrigger><SelectValue placeholder="Select connection" /></SelectTrigger>
                    <SelectContent>{connectionOptions.map((name) => <SelectItem key={name} value={name}>{name}</SelectItem>)}</SelectContent>
                  </Select>
                </Field>
              </CardContent>
            </Card>

            <Card>
              <CardHeader><CardTitle>Command console</CardTitle><CardDescription>POST JSON to /api/v1/connections/:name/commands.</CardDescription></CardHeader>
              <CardContent className="space-y-4">
                <Textarea className="json-editor min-h-64" value={payload} onChange={(e) => setPayload(e.target.value)} spellCheck={false} />
                <div className="grid gap-3 md:grid-cols-[1fr_10rem]">
                  <label className="flex items-center gap-2 text-sm"><Checkbox checked={waitForResponse} onCheckedChange={(checked) => setWaitForResponse(Boolean(checked))} /> waitForResponse</label>
                  <Input inputMode="numeric" value={timeoutMs} onChange={(e) => setTimeoutMs(e.target.value)} disabled={!waitForResponse} />
                </div>
                <div className="flex flex-wrap gap-2"><Button onClick={() => void sendCommand()}><Send className="mr-2 h-4 w-4" />Send command</Button><Button variant="secondary" onClick={savePreset}><Save className="mr-2 h-4 w-4" />Save as preset</Button></div>
              </CardContent>
            </Card>
          </TabsContent>

          <TabsContent value="events">
            <Card><CardHeader><CardTitle>Events log</CardTitle><CardDescription>Live SSE from /api/v1/events with in-memory history replay.</CardDescription></CardHeader><CardContent>
              <ScrollArea className="h-96 rounded-md border bg-black/25 p-3">
                {events.length === 0 ? <p className="text-sm text-muted-foreground">No events recorded yet. Send commands or connect a device to populate this log.</p> : events.map((item) => <pre key={item.id} className="mb-3 whitespace-pre-wrap rounded-md bg-muted/30 p-3 text-xs"><span className="text-sky-300">{item.event}</span>{"\n"}{item.data}</pre>)}
              </ScrollArea>
            </CardContent></Card>
          </TabsContent>

          <TabsContent value="presets" className="grid gap-4 lg:grid-cols-[22rem_1fr]">
            <Card><CardHeader><CardTitle>Create preset</CardTitle><CardDescription>Save the current JSON payload for reuse.</CardDescription></CardHeader><CardContent className="space-y-3"><Field label="Preset name"><Input value={presetName} onChange={(e) => setPresetName(e.target.value)} /></Field><Button onClick={savePreset}><Save className="mr-2 h-4 w-4" />Save preset</Button></CardContent></Card>
            <Card><CardHeader><CardTitle>Saved presets</CardTitle><CardDescription>Load, send, or delete stored command payloads.</CardDescription></CardHeader><CardContent className="space-y-3">
              {presets.length === 0 ? <p className="text-sm text-muted-foreground">No presets saved.</p> : presets.map((preset) => <div key={preset.id} className="rounded-lg border p-3"><div className="flex flex-wrap items-center justify-between gap-2"><div><div className="font-medium">{preset.name}</div><div className="text-xs text-muted-foreground">#{preset.id}</div></div><div className="flex gap-2"><Button size="sm" variant="secondary" onClick={() => applyPreset(preset)}><Play className="mr-1 h-3 w-3" />Load</Button><Button size="sm" onClick={() => void sendCommand(preset.payload)}>Send</Button><Button size="sm" variant="destructive" onClick={() => void removePreset(preset.id)}><Trash2 className="h-3 w-3" /></Button></div></div><pre className="mt-3 max-h-32 overflow-auto rounded bg-muted/30 p-2 text-xs">{JSON.stringify(preset.payload, null, 2)}</pre></div>)}
            </CardContent></Card>
          </TabsContent>
        </Tabs>

        <Card>
          <CardHeader><CardTitle>Serial ports</CardTitle><CardDescription>Ports depend on host hardware and OS permissions.</CardDescription></CardHeader>
          <CardContent>{ports.length === 0 ? <p className="text-sm text-muted-foreground">No ports reported by the API.</p> : <Table><TableHeader><TableRow><TableHead>Name</TableHead><TableHead>Type</TableHead><TableHead>Manufacturer</TableHead><TableHead>Serial</TableHead></TableRow></TableHeader><TableBody>{ports.map((port) => <TableRow key={port.name}><TableCell>{port.name}</TableCell><TableCell>{port.type ?? port.port_type ?? "unknown"}</TableCell><TableCell>{port.manufacturer ?? "—"}</TableCell><TableCell>{port.serial_number ?? "—"}</TableCell></TableRow>)}</TableBody></Table>}</CardContent>
        </Card>
      </div>
    </main>
  );
}

function StatusCard({ icon, label, value }: { icon: React.ReactNode; label: string; value: string }) {
  return <Card><CardContent className="flex items-center gap-3 p-4"><div className="rounded-md bg-sky-500/10 p-2 text-sky-300">{icon}</div><div><div className="metric-label">{label}</div><div className="text-sm font-medium">{value}</div></div></CardContent></Card>;
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return <div className="space-y-2"><Label>{label}</Label>{children}</div>;
}

function decodeDelimiter(value: string) {
  return value.replace(/\\r/g, "\r").replace(/\\n/g, "\n").replace(/\\t/g, "\t");
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

export default App;
