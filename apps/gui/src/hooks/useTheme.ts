import { useEffect, useState } from "react";

export type TakokitTheme = "dark" | "light";

const STORAGE_KEY = "takokit.gui.theme";

function initialTheme(): TakokitTheme {
  const stored = window.localStorage.getItem(STORAGE_KEY);
  return stored === "light" || stored === "dark" ? stored : "dark";
}

export function useTheme() {
  const [theme, setTheme] = useState<TakokitTheme>(() => initialTheme());

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    document.documentElement.style.colorScheme = theme;
    window.localStorage.setItem(STORAGE_KEY, theme);
  }, [theme]);

  return {
    theme,
    toggleTheme: () => setTheme((current) => current === "dark" ? "light" : "dark")
  };
}
