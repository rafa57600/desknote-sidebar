import type { Edge } from "./native";

export interface Note {
  id: string;
  title: string;
  body: string;
  createdAt: number;
  updatedAt: number;
}

export interface SidebarSettings {
  width: number;
  expandedWidth: number;
  collapsed: boolean;
  edge: Edge;
  alwaysOnTop: boolean;
  monitorId: string | null;
}

const NOTES_KEY = "desknote.notes.v1";
const SETTINGS_KEY = "desknote.settings.v1";

export const DEFAULT_SETTINGS: SidebarSettings = {
  width: 360,
  expandedWidth: 360,
  collapsed: false,
  edge: "right",
  alwaysOnTop: true,
  monitorId: null,
};

const makeStarterNote = (): Note => ({
  id: crypto.randomUUID(),
  title: "Welcome",
  body: "This sidebar reserves desktop space on Windows. Add notes here and maximize another app to verify it stops at the sidebar edge.",
  createdAt: Date.now(),
  updatedAt: Date.now(),
});

export function loadNotes(): Note[] {
  try {
    const raw = localStorage.getItem(NOTES_KEY);
    if (!raw) return [makeStarterNote()];
    const parsed = JSON.parse(raw) as Note[];
    return Array.isArray(parsed) && parsed.length > 0 ? parsed : [makeStarterNote()];
  } catch {
    return [makeStarterNote()];
  }
}

export function saveNotes(notes: Note[]) {
  localStorage.setItem(NOTES_KEY, JSON.stringify(notes));
}

export function loadSettings(): SidebarSettings {
  try {
    const raw = localStorage.getItem(SETTINGS_KEY);
    if (!raw) return DEFAULT_SETTINGS;
    const value = { ...DEFAULT_SETTINGS, ...(JSON.parse(raw) as Partial<SidebarSettings>) };
    value.width = clamp(value.width, 56, 720);
    value.expandedWidth = clamp(value.expandedWidth, 280, 720);
    value.edge = value.edge === "left" ? "left" : "right";
    return value;
  } catch {
    return DEFAULT_SETTINGS;
  }
}

export function saveSettings(settings: SidebarSettings) {
  localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
}

export function clamp(value: number, min: number, max: number) {
  return Math.min(max, Math.max(min, value));
}
