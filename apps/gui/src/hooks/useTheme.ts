import { useEffect, useState } from "react";

export type TakokitTheme = "dark" | "light";

const STORAGE_KEY = "takokit.gui.theme";
const THEME_EVENT = "takokit-theme-change";

function initialTheme(): TakokitTheme {
  const stored = window.localStorage.getItem(STORAGE_KEY);
  return stored === "light" || stored === "dark" ? stored : "dark";
}

function applyTheme(theme: TakokitTheme) {
  document.documentElement.dataset.theme = theme;
  document.documentElement.style.colorScheme = theme;
  window.localStorage.setItem(STORAGE_KEY, theme);
}

export function useTheme() {
  const [theme, setLocalTheme] = useState<TakokitTheme>(() => initialTheme());

  useEffect(() => {
    applyTheme(theme);
  }, [theme]);

  useEffect(() => {
    const sync = (event: Event) => {
      const next = (event as CustomEvent<TakokitTheme>).detail;
      if (next === "dark" || next === "light") setLocalTheme(next);
    };
    window.addEventListener(THEME_EVENT, sync);
    return () => window.removeEventListener(THEME_EVENT, sync);
  }, []);

  function setTheme(next: TakokitTheme) {
    applyTheme(next);
    setLocalTheme(next);
    window.dispatchEvent(new CustomEvent<TakokitTheme>(THEME_EVENT, { detail: next }));
  }

  return {
    theme,
    setTheme,
    toggleTheme: () => setTheme(theme === "dark" ? "light" : "dark")
  };
}
