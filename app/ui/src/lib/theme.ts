import { ipc } from './ipc';

interface ThemeFile {
  fontUI?: string;
  fontMono?: string;
  shared?: Record<string, string>;
  light?: Record<string, string>;
  dark?: Record<string, string>;
}

const STYLE_ID = 'shelf-theme-overrides';

function declarations(map: Record<string, string> | undefined): string {
  if (!map) return '';
  return Object.entries(map)
    .filter(([key, value]) => /^--[a-z0-9-]+$/i.test(key) && typeof value === 'string')
    .map(([key, value]) => `${key}: ${value};`)
    .join('\n');
}

export function applyThemeJson(raw: string): void {
  const theme = JSON.parse(raw) as ThemeFile;
  const shared = declarations(theme.shared);
  const light = declarations(theme.light);
  const dark = declarations(theme.dark);
  const fonts = [
    theme.fontUI ? `--font-ui: ${theme.fontUI};` : '',
    theme.fontMono ? `--font-mono: ${theme.fontMono};` : '',
  ]
    .filter(Boolean)
    .join('\n');

  const css = `
:root {
${fonts}
${shared}
${light}
}
@media (prefers-color-scheme: dark) {
  :root:not([data-theme='light']) {
${dark}
  }
}
:root[data-theme='dark'] {
${dark}
}
`;
  let style = document.getElementById(STYLE_ID) as HTMLStyleElement | null;
  if (!style) {
    style = document.createElement('style');
    style.id = STYLE_ID;
    document.head.appendChild(style);
  }
  style.textContent = css;
}

export async function loadThemeFile(): Promise<string> {
  const raw = await ipc.readTheme();
  applyThemeJson(raw);
  return raw;
}
