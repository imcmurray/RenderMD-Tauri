// Apply an Omarchy colors.toml palette to the shell chrome.

export interface OmarchyPalette {
  dark: boolean;
  background: string;
  darkerBackground: string;
  lighterBackground: string;
  foreground: string;
  darkForeground: string;
  accent: string;
  selection: string;
  muted: string;
  red: string;
  yellow: string;
  green: string;
  cyan: string;
  blue: string;
  magenta: string;
}

export function applyOmarchyPalette(p: OmarchyPalette) {
  const r = document.documentElement;
  const set = (name: string, value: string) => r.style.setProperty(name, value);
  set("--hb-bg", p.background);
  set("--hb-bg-raised", p.lighterBackground);
  set("--hb-fg", p.foreground);
  set("--hb-muted", p.muted || p.darkForeground);
  set("--accent", p.accent);
  set("--hb-border", p.muted);
  set("--hb-selection", p.selection);
  set("--hb-red", p.red);
  r.dataset.theme = p.dark ? "dark" : "light";
  r.classList.add("omarchy");
}
