import { useCallback, useEffect, useMemo, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { clamp, loadNotes, loadSettings, saveNotes, saveSettings, type Note, type SidebarSettings } from "./storage";
import { native, type AppBarState, type Edge, type MonitorInfo } from "./native";

const COLLAPSED_WIDTH = 56;
const MIN_WIDTH = 280;
const MAX_WIDTH = 720;
const desktopWindow = getCurrentWindow();

function App() {
  const [notes, setNotes] = useState<Note[]>(loadNotes);
  const [settings, setSettings] = useState<SidebarSettings>(loadSettings);
  const [nativeState, setNativeState] = useState<AppBarState | null>(null);
  const [monitors, setMonitors] = useState<MonitorInfo[]>([]);
  const [query, setQuery] = useState("");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [status, setStatus] = useState("Connecting to Windows Shell…");
  const settingsRef = useRef(settings);

  useEffect(() => {
    settingsRef.current = settings;
    saveSettings(settings);
  }, [settings]);

  useEffect(() => saveNotes(notes), [notes]);

  const refreshMonitors = useCallback(async () => {
    try {
      const next = await native.listMonitors();
      setMonitors(next);
      return next;
    } catch (error) {
      console.error(error);
      return [] as MonitorInfo[];
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const initial = await native.getState();
        if (cancelled) return;
        setNativeState(initial);

        const available = await refreshMonitors();
        const saved = settingsRef.current;
        const effectiveWidth = saved.collapsed ? COLLAPSED_WIDTH : saved.expandedWidth;

        await native.setEdge(saved.edge);
        await native.setAlwaysOnTop(saved.alwaysOnTop);

        const preferred = available.find((monitor) => monitor.id === saved.monitorId);
        if (preferred) {
          await native.setMonitor(preferred.x + Math.floor(preferred.width / 2), preferred.y + Math.floor(preferred.height / 2));
        }

        const finalState = await native.setWidth(effectiveWidth);
        if (cancelled) return;
        setNativeState(finalState);
        setSettings((current) => ({ ...current, width: effectiveWidth }));
        setStatus(finalState.supported && finalState.registered ? "Docked — desktop space reserved" : "Preview mode — Windows AppBar unavailable");
      } catch (error) {
        console.error(error);
        if (!cancelled) setStatus("Native AppBar unavailable");
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [refreshMonitors]);

  const filteredNotes = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return notes;
    return notes.filter((note) => `${note.title}\n${note.body}`.toLowerCase().includes(q));
  }, [notes, query]);

  const updateNote = useCallback((id: string, patch: Partial<Pick<Note, "title" | "body">>) => {
    setNotes((current) =>
      current.map((note) => note.id === id ? { ...note, ...patch, updatedAt: Date.now() } : note),
    );
  }, []);

  const addNote = useCallback(() => {
    const now = Date.now();
    setNotes((current) => [
      { id: crypto.randomUUID(), title: "", body: "", createdAt: now, updatedAt: now },
      ...current,
    ]);
  }, []);

  const deleteNote = useCallback((id: string) => {
    setNotes((current) => current.filter((note) => note.id !== id));
  }, []);

  const setWidth = useCallback((width: number) => {
    const next = clamp(width, MIN_WIDTH, MAX_WIDTH);
    setSettings((current) => ({ ...current, width: next, expandedWidth: next, collapsed: false }));
    void native.setWidth(next).then(setNativeState).catch(console.error);
  }, []);

  const toggleCollapsed = useCallback(async () => {
    const current = settingsRef.current;
    const nextCollapsed = !current.collapsed;
    const nextWidth = nextCollapsed ? COLLAPSED_WIDTH : current.expandedWidth;
    setSettings((value) => ({ ...value, collapsed: nextCollapsed, width: nextWidth }));
    try {
      setNativeState(await native.setWidth(nextWidth));
    } catch (error) {
      console.error(error);
    }
  }, []);

  const changeEdge = useCallback(async (edge: Edge) => {
    setSettings((current) => ({ ...current, edge }));
    try {
      setNativeState(await native.setEdge(edge));
    } catch (error) {
      console.error(error);
    }
  }, []);

  const changeTopmost = useCallback(async (enabled: boolean) => {
    setSettings((current) => ({ ...current, alwaysOnTop: enabled }));
    try {
      setNativeState(await native.setAlwaysOnTop(enabled));
    } catch (error) {
      console.error(error);
    }
  }, []);

  const selectMonitor = useCallback(async (id: string) => {
    const monitor = monitors.find((item) => item.id === id);
    if (!monitor) return;
    setSettings((current) => ({ ...current, monitorId: id }));
    try {
      setNativeState(await native.setMonitor(
        monitor.x + Math.floor(monitor.width / 2),
        monitor.y + Math.floor(monitor.height / 2),
      ));
    } catch (error) {
      console.error(error);
    }
  }, [monitors]);

  const syncNativeWidth = useCallback(async () => {
    try {
      const next = await native.getState();
      setNativeState(next);
      if (!settingsRef.current.collapsed && next.width >= MIN_WIDTH) {
        setSettings((current) => ({ ...current, width: next.width, expandedWidth: next.width }));
      }
    } catch (error) {
      console.error(error);
    }
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let timer: number | undefined;
    void desktopWindow.onResized(() => {
      if (timer !== undefined) window.clearTimeout(timer);
      timer = window.setTimeout(() => void syncNativeWidth(), 90);
    }).then((fn) => { unlisten = fn; });

    return () => {
      if (timer !== undefined) window.clearTimeout(timer);
      unlisten?.();
    };
  }, [syncNativeWidth]);

  const beginResize = useCallback(async (event: ReactPointerEvent<HTMLDivElement>) => {
    if (settingsRef.current.collapsed || event.button !== 0) return;
    event.preventDefault();

    const direction = settingsRef.current.edge === "right" ? "West" as const : "East" as const;
    try {
      await desktopWindow.startResizeDragging(direction);
    } catch (error) {
      console.error(error);
    }
  }, []);

  const savedMonitor = monitors.find((monitor) => monitor.id === settings.monitorId);
  const currentMonitorId = savedMonitor?.id ?? monitors.find((monitor) => monitor.is_current)?.id ?? monitors[0]?.id ?? "";
  const appClass = `app edge-${settings.edge} ${settings.collapsed ? "collapsed" : ""}`;

  return (
    <div className={appClass}>
      <div className="resize-handle" onPointerDown={(event) => void beginResize(event)} aria-label="Resize sidebar" />

      {settings.collapsed ? (
        <CollapsedRail
          onExpand={toggleCollapsed}
          onAdd={() => { void toggleCollapsed(); addNote(); }}
          onSettings={() => { void toggleCollapsed(); setSettingsOpen(true); }}
          onExit={() => void native.exit()}
        />
      ) : (
        <>
          <header className="topbar">
            <div>
              <div className="brand-row">
                <span className="brand-mark">D</span>
                <strong>DeskNote</strong>
              </div>
              <div className="status"><span className={nativeState?.registered ? "dot active" : "dot"} />{status}</div>
            </div>
            <div className="window-actions">
              <button className="icon-button" title="Collapse" onClick={() => void toggleCollapsed()}>›</button>
              <button className="icon-button" title="Settings" onClick={() => setSettingsOpen((value) => !value)}>⚙</button>
              <button className="icon-button danger" title="Exit" onClick={() => void native.exit()}>×</button>
            </div>
          </header>

          <div className="toolbar">
            <input
              className="search"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="Search notes"
              aria-label="Search notes"
            />
            <button className="primary" onClick={addNote}>+ Note</button>
          </div>

          {settingsOpen && (
            <SettingsPanel
              settings={settings}
              monitors={monitors}
              currentMonitorId={currentMonitorId}
              onWidth={setWidth}
              onEdge={changeEdge}
              onAlwaysOnTop={changeTopmost}
              onMonitor={selectMonitor}
              onRefreshMonitors={() => void refreshMonitors()}
              onRefreshPosition={() => void native.refreshPosition().then(setNativeState).catch(console.error)}
            />
          )}

          <main className="notes-scroll">
            {filteredNotes.length === 0 ? (
              <div className="empty-state">No notes match “{query}”.</div>
            ) : filteredNotes.map((note) => (
              <article className="note-card" key={note.id}>
                <div className="note-header">
                  <input
                    value={note.title}
                    onChange={(event) => updateNote(note.id, { title: event.target.value })}
                    placeholder="Untitled note"
                    aria-label="Note title"
                  />
                  <button className="delete-note" title="Delete note" onClick={() => deleteNote(note.id)}>×</button>
                </div>
                <textarea
                  value={note.body}
                  onChange={(event) => updateNote(note.id, { body: event.target.value })}
                  placeholder="Write something…"
                  aria-label="Note text"
                />
                <footer>{new Date(note.updatedAt).toLocaleString()}</footer>
              </article>
            ))}
          </main>
        </>
      )}
    </div>
  );
}

function CollapsedRail(props: { onExpand: () => void; onAdd: () => void; onSettings: () => void; onExit: () => void }) {
  return (
    <div className="collapsed-rail">
      <button className="rail-brand" onClick={props.onExpand} title="Expand">D</button>
      <div className="rail-actions">
        <button onClick={props.onAdd} title="New note">+</button>
        <button onClick={props.onSettings} title="Settings">⚙</button>
      </div>
      <button className="rail-exit" onClick={props.onExit} title="Exit">×</button>
    </div>
  );
}

function SettingsPanel(props: {
  settings: SidebarSettings;
  monitors: MonitorInfo[];
  currentMonitorId: string;
  onWidth: (width: number) => void;
  onEdge: (edge: Edge) => void;
  onAlwaysOnTop: (enabled: boolean) => void;
  onMonitor: (id: string) => void;
  onRefreshMonitors: () => void;
  onRefreshPosition: () => void;
}) {
  return (
    <section className="settings-panel">
      <div className="settings-title"><strong>Sidebar settings</strong><span>Windows AppBar</span></div>

      <label className="setting-row">
        <span>Width <small>{props.settings.expandedWidth}px</small></span>
        <input
          type="range"
          min={MIN_WIDTH}
          max={MAX_WIDTH}
          step={4}
          value={props.settings.expandedWidth}
          onChange={(event) => props.onWidth(Number(event.target.value))}
        />
      </label>

      <div className="setting-row inline">
        <span>Dock edge</span>
        <div className="segmented">
          <button className={props.settings.edge === "left" ? "selected" : ""} onClick={() => props.onEdge("left")}>Left</button>
          <button className={props.settings.edge === "right" ? "selected" : ""} onClick={() => props.onEdge("right")}>Right</button>
        </div>
      </div>

      <label className="setting-row inline">
        <span>Always on top</span>
        <input type="checkbox" checked={props.settings.alwaysOnTop} onChange={(event) => props.onAlwaysOnTop(event.target.checked)} />
      </label>

      <label className="setting-row">
        <span>Monitor</span>
        <div className="monitor-row">
          <select value={props.currentMonitorId} onChange={(event) => props.onMonitor(event.target.value)}>
            {props.monitors.map((monitor, index) => (
              <option key={monitor.id} value={monitor.id}>
                {monitor.name || `Monitor ${index + 1}`} — {monitor.width}×{monitor.height} ({Math.round(monitor.scale_factor * 100)}%)
              </option>
            ))}
          </select>
          <button onClick={props.onRefreshMonitors} title="Refresh monitors">↻</button>
        </div>
      </label>

      <button className="secondary full" onClick={props.onRefreshPosition}>Recalculate reserved area</button>
    </section>
  );
}

export default App;
