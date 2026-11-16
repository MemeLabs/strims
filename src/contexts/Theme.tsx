// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, {
  ReactNode,
  createContext,
  useContext,
  useLayoutEffect,
  useMemo,
  useState,
} from "react";

import { readItem, writeItem } from "../lib/storage";

type ColorScheme = "dark" | "light";

// The color scheme the device last chose, kept in local storage.
const THEME_STORAGE_KEY = "theme";

const storedColorScheme = (): ColorScheme | null => {
  const stored = readItem(THEME_STORAGE_KEY);
  return stored === "dark" || stored === "light" ? stored : null;
};

export interface ThemeState {
  colorScheme: ColorScheme;
  setColorScheme: React.Dispatch<React.SetStateAction<ColorScheme>>;
}

const ThemeContext = createContext<ThemeState>(null);

// Each mounted provider's color by the order it first rendered in. The
// browser's theme color and the page's background are the latest's, an inner
// provider's over its outer one's.
const themeColors = new Map<number, string>();
let nextProvider = 0;
let themeTags: { meta: HTMLMetaElement; style: HTMLStyleElement } | null = null;

const applyThemeColor = () => {
  if (themeColors.size === 0) {
    themeTags?.meta.remove();
    themeTags?.style.remove();
    themeTags = null;
    return;
  }
  const color = themeColors.get(Math.max(...themeColors.keys()));
  if (!themeTags) {
    themeTags = { meta: document.createElement("meta"), style: document.createElement("style") };
    themeTags.meta.name = "theme-color";
    document.head.append(themeTags.meta, themeTags.style);
  }
  themeTags.meta.content = color;
  themeTags.style.textContent = `html, body { background: ${color}; }`;
};

const useThemeColor = (color: string) => {
  const id = useMemo(() => nextProvider++, []);
  useLayoutEffect(() => {
    themeColors.set(id, color);
    applyThemeColor();
  }, [color]);
  useLayoutEffect(
    () => () => {
      themeColors.delete(id);
      applyThemeColor();
    },
    []
  );
};

interface ProviderProps {
  children: ReactNode;
}

export const Provider: React.FC<ProviderProps> = ({ children }) => {
  const [colorScheme, setColorScheme] = useState<ColorScheme>(() => storedColorScheme() ?? "dark");

  // if (!savedTheme && window.matchMedia) {
  //   useEffect(() => {
  //     const query = window.matchMedia("(prefers-color-scheme: dark)");

  //     setColorScheme(query.matches ? "dark" : "light");

  //     const handleChange = (e: MediaQueryListEvent) => setColorScheme(e.matches ? "dark" : "light");
  //     query.addEventListener("change", handleChange);
  //     return () => query.removeEventListener("change", handleChange);
  //   }, []);
  // }

  // TODO: load from... config? webpack?
  const themeColor = colorScheme === "dark" ? "#222933" : "#efeff1";
  useThemeColor(themeColor);

  const value = useMemo(
    () => ({
      colorScheme,
      setColorScheme: (v: ColorScheme) => {
        writeItem(THEME_STORAGE_KEY, v);
        setColorScheme(v);
      },
    }),
    [colorScheme]
  );

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
};

Provider.displayName = "Theme.Provider";

export const useTheme = (): ThemeState => useContext(ThemeContext);
