import React, { createContext, useContext, useMemo, useState } from "react";
import { Helmet } from "react-helmet";

type ColorScheme = "dark" | "light";

export interface ThemeState {
  colorScheme: ColorScheme;
  setColorScheme: React.Dispatch<React.SetStateAction<ColorScheme>>;
}

const ThemeContext = createContext<ThemeState>(null);

export const Provider: React.FC = ({ children }) => {
  const savedTheme = useMemo(() => window.localStorage.getItem("theme") as ColorScheme, []);
  const [colorScheme, setColorScheme] = useState<ColorScheme>(savedTheme || "dark");

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
  const themeColor = colorScheme === "dark" ? "#222933" : "#d5d5d5";

  const value = useMemo(
    () => ({
      colorScheme,
      setColorScheme: (v: ColorScheme) => {
        window.localStorage.setItem("theme", v);
        setColorScheme(v);
      },
    }),
    [colorScheme]
  );

  return (
    <ThemeContext.Provider value={value}>
      <Helmet>
        <meta name="theme-color" content={themeColor} />
        <style type="text/css">{`html, body { background: ${themeColor}; }`}</style>
      </Helmet>
      {children}
    </ThemeContext.Provider>
  );
};

Provider.displayName = "Theme.Provider";

export const useTheme = (): ThemeState => useContext(ThemeContext);
