import { invoke } from "@tauri-apps/api/core";

export type Edge = "left" | "right";

export interface AppBarState {
  supported: boolean;
  registered: boolean;
  width: number;
  edge: Edge;
  always_on_top: boolean;
}

export interface MonitorInfo {
  id: string;
  name: string;
  x: number;
  y: number;
  width: number;
  height: number;
  scale_factor: number;
  is_current: boolean;
}

export const native = {
  getState: () => invoke<AppBarState>("get_appbar_state"),
  setWidth: (width: number) =>
    invoke<AppBarState>("set_sidebar_width", { width: Math.round(width) }),
  setEdge: (edge: Edge) => invoke<AppBarState>("set_sidebar_edge", { edge }),
  setAlwaysOnTop: (enabled: boolean) =>
    invoke<AppBarState>("set_sidebar_always_on_top", { enabled }),
  listMonitors: () => invoke<MonitorInfo[]>("list_monitors"),
  setMonitor: (x: number, y: number) =>
    invoke<AppBarState>("set_sidebar_monitor", { x, y }),
  refreshPosition: () => invoke<AppBarState>("refresh_sidebar_position"),
  exit: () => invoke<void>("exit_app"),
};
