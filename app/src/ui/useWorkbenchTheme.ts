import { useEffect, useState } from 'react';

export type ThemePreference = 'system' | 'light' | 'dark';
const THEME_KEY = 'boardstudio:v2:theme';

const readThemePreference = (): ThemePreference => {
  try {
    const stored = localStorage.getItem(THEME_KEY);
    return stored === 'light' || stored === 'dark' ? stored : 'system';
  } catch {
    return 'system';
  }
};

const systemColorScheme = (): 'light' | 'dark' => {
  try {
    return matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  } catch {
    return 'light';
  }
};

export function useWorkbenchTheme() {
  const [themePreference, setThemePreference] = useState<ThemePreference>(readThemePreference);
  const [systemScheme, setSystemScheme] = useState(systemColorScheme);
  const colorScheme = themePreference === 'system' ? systemScheme : themePreference;

  useEffect(() => {
    const media = matchMedia('(prefers-color-scheme: dark)');
    const update = (event: MediaQueryListEvent) => setSystemScheme(event.matches ? 'dark' : 'light');
    if (media.addEventListener) {
      media.addEventListener('change', update);
      return () => media.removeEventListener('change', update);
    }
    media.addListener(update);
    return () => media.removeListener(update);
  }, []);

  useEffect(() => {
    window.document.documentElement.dataset.theme = colorScheme;
    window.document.documentElement.style.colorScheme = colorScheme;
    const meta = window.document.querySelector<HTMLMetaElement>('meta[name="theme-color"]');
    if (meta) meta.content = colorScheme === 'dark' ? '#151c26' : '#f7f8fa';
  }, [colorScheme]);

  const chooseTheme = (preference: ThemePreference) => {
    setThemePreference(preference);
    try { localStorage.setItem(THEME_KEY, preference); } catch { /* Keep the current session usable. */ }
  };

  return { themePreference, colorScheme, chooseTheme };
}
