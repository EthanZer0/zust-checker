// 主题管理（深色 / 浅色），持久化到 localStorage
// 使用 Svelte 5 runes 模块，组件读取 themeState.current 即可获得响应式更新

export type Theme = 'dark' | 'light';

const STORAGE_KEY = 'zust-theme';

function initialTheme(): Theme {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === 'light' || saved === 'dark') return saved;
  } catch (_) {}
  return 'dark'; // 默认深色
}

export const themeState = $state<{ current: Theme }>({ current: initialTheme() });

/** 将当前主题写入 <html data-theme="..."> */
export function applyTheme() {
  document.documentElement.setAttribute('data-theme', themeState.current);
}

/** 切换主题并持久化 */
export function toggleTheme() {
  themeState.current = themeState.current === 'dark' ? 'light' : 'dark';
  try { localStorage.setItem(STORAGE_KEY, themeState.current); } catch (_) {}
  applyTheme();
}
