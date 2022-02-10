import React, { createContext, useContext, useEffect, useMemo, useState } from "react";

type ColorScheme = "dark" | "light";

export interface ThemeState {
  colorScheme: ColorScheme;
  setColorScheme: React.Dispatch<React.SetStateAction<ColorScheme>>;
}

const ThemeContext = createContext<ThemeState>(null);

export const Provider: React.FC = ({ children }) => {
  const savedTheme = useMemo(() => window.localStorage.getItem("theme") as ColorScheme, []);
  const [colorScheme, setColorScheme] = useState<ColorScheme>(savedTheme || "dark");

  if (!savedTheme && window.matchMedia) {
    useEffect(() => {
      const query = window.matchMedia("(prefers-color-scheme: dark)");

      setColorScheme(query.matches ? "dark" : "light");

      const handleChange = (e: MediaQueryListEvent) => setColorScheme(e.matches ? "dark" : "light");
      query.addEventListener("change", handleChange);
      return () => query.removeEventListener("change", handleChange);
    }, []);
  }

  const value = useMemo(
    () => ({
      colorScheme,
      setColorScheme,
    }),
    [colorScheme]
  );

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
};

Provider.displayName = "Theme.Provider";

export const useTheme = (): ThemeState => useContext(ThemeContext);
