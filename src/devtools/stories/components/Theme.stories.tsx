// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Theme.stories.scss";

import clsx from "clsx";
import { groupBy } from "lodash";
import React, { ReactNode, useLayoutEffect, useRef, useState } from "react";

import { WithThemeProps, withTheme } from "../../../components/Theme";
import { useTheme } from "../../../contexts/Theme";
import { Story } from "../../storybook";

type Scope = "theme" | "dark" | "light";

const scopes: Record<string, Scope> = {
  ".theme": "theme",
  ".theme--dark": "dark",
  ".theme--light": "light",
};

interface Token {
  name: string;
  /** The rules setting it: .theme's (every theme), or a color scheme's. */
  scopes: Set<Scope>;
}

// Every custom property Theme.scss sets on .theme, .theme--dark and
// .theme--light, in the stylesheets' order.
const readTokens = (): Token[] => {
  const tokens = new Map<string, Token>();
  for (const sheet of Array.from(document.styleSheets)) {
    let rules: CSSRuleList;
    try {
      rules = sheet.cssRules;
    } catch {
      continue;
    }
    for (const rule of Array.from(rules)) {
      const scope = rule instanceof CSSStyleRule ? scopes[rule.selectorText] : undefined;
      if (!scope) {
        continue;
      }
      const { style } = rule as CSSStyleRule;
      for (let i = 0; i < style.length; i++) {
        const name = style[i];
        if (name.startsWith("--")) {
          const token = tokens.get(name) ?? { name, scopes: new Set() };
          token.scopes.add(scope);
          tokens.set(name, token);
        }
      }
    }
  }
  return [...tokens.values()];
};

interface Value extends Token {
  /** Its computed value in the current theme; "" when unset there. */
  value: string;
}

// The tokens with their values under the storybook's theme, read again when
// the toolbar switches it.
const useTokens = (): [React.RefObject<HTMLDivElement>, Value[], Scope] => {
  const ref = useRef<HTMLDivElement>(null);
  const { colorScheme } = useTheme();
  const [values, setValues] = useState<Value[]>([]);
  useLayoutEffect(() => {
    const style = getComputedStyle(ref.current);
    setValues(readTokens().map((t) => ({ ...t, value: style.getPropertyValue(t.name).trim() })));
  }, [colorScheme]);
  return [ref, values, colorScheme];
};

const isColor = (v: string) => v !== "" && CSS.supports("color", v);

// Where a themed token comes from: both schemes, only the current one, or
// only the other (unset here).
const origin = ({ scopes }: Token, scheme: Scope) => {
  if (scopes.has("dark") && scopes.has("light")) {
    return null;
  }
  return scopes.has(scheme) ? `${scheme} only` : `unset in ${scheme}`;
};

const Swatch: React.FC<{ token: Value; scheme: Scope }> = ({ token, scheme }) => {
  const note = token.scopes.has("theme") ? null : origin(token, scheme);
  return (
    <div className="theme_story__swatch">
      <div
        className={
          isColor(token.value) ? "theme_story__chip" : "theme_story__chip theme_story__chip--none"
        }
        style={{ "--chip": `var(${token.name})` } as React.CSSProperties}
      />
      <div className="theme_story__label">
        <code className="theme_story__name">{token.name}</code>
        <code className="theme_story__value">{token.value || "unset"}</code>
        {note && <span className="theme_story__note">{note}</span>}
      </div>
    </div>
  );
};

const Section: React.FC<{ title: string; count: number; children: ReactNode }> = ({
  title,
  count,
  children,
}) => (
  <section className="theme_story__section">
    <h3 className="theme_story__title">
      {title} <span className="theme_story__count">({count})</span>
    </h3>
    {children}
  </section>
);

interface PageProps {
  children: (values: Value[], scheme: Scope) => ReactNode;
}

// withTheme gives the page the theme's classes, which set the tokens.
const Page = withTheme<PageProps>(({ className, children }: PageProps & WithThemeProps) => {
  const [ref, values, scheme] = useTokens();
  return (
    <div ref={ref} className={clsx(className, "theme_story")}>
      <p className="theme_story__caption">
        components/Theme.scss&apos;s custom properties as the {scheme} theme computes them.
      </p>
      {values.length > 0 && children(values, scheme)}
    </div>
  );
});

// A token's family: its name without a trailing step ("--color-slate-4": "--color-slate").
const family = (name: string) => name.replace(/-\d+$/, "");

const PaletteStory: React.FC = () => (
  <Page>
    {(values, scheme) => {
      const palette = values.filter((t) => t.scopes.has("theme") && isColor(t.value));
      const families = groupBy(palette, (t) => family(t.name));
      return (
        <Section title="Palette, every theme" count={palette.length}>
          {Object.entries(families).map(([name, tokens]) => (
            <div key={name} className="theme_story__family">
              <code className="theme_story__family_name">{name}</code>
              <div className="theme_story__grid">
                {tokens.map((t) => (
                  <Swatch key={t.name} token={t} scheme={scheme} />
                ))}
              </div>
            </div>
          ))}
        </Section>
      );
    }}
  </Page>
);

const isShadow = (t: Token) => t.name.startsWith("--shadow-");

const ThemedStory: React.FC = () => (
  <Page>
    {(values, scheme) => {
      const themed = values.filter((t) => !t.scopes.has("theme") && !isShadow(t));
      const colors = themed.filter(
        (t) => isColor(t.value) || (!t.value && t.name.startsWith("--color"))
      );
      const others = themed.filter((t) => !colors.includes(t));
      return (
        <>
          <Section title="Theme colors" count={colors.length}>
            <div className="theme_story__grid">
              {colors.map((t) => (
                <Swatch key={t.name} token={t} scheme={scheme} />
              ))}
            </div>
          </Section>
          {others.length > 0 && (
            <Section title="Other themed values" count={others.length}>
              <div className="theme_story__grid">
                {others.map((t) => (
                  <Swatch key={t.name} token={t} scheme={scheme} />
                ))}
              </div>
            </Section>
          )}
        </>
      );
    }}
  </Page>
);

const ElevationStory: React.FC = () => (
  <Page>
    {(values) => {
      const shadows = values.filter(isShadow);
      return (
        <Section title="Elevation" count={shadows.length}>
          <div className="theme_story__shadows">
            {shadows.map((t) => (
              <div
                key={t.name}
                className="theme_story__shadow"
                style={{ boxShadow: `var(${t.name})` }}
              >
                <code className="theme_story__name">{t.name}</code>
                <code className="theme_story__value">{t.value || "unset"}</code>
              </div>
            ))}
          </div>
        </Section>
      );
    }}
  </Page>
);

const scaleSample = (t: Value): ReactNode => {
  if (t.name.startsWith("--font-size-")) {
    return <span style={{ fontSize: `var(${t.name})` }}>Aa</span>;
  }
  if (t.name.startsWith("--font-weight-")) {
    return <span style={{ fontWeight: `var(${t.name})` }}>Aa</span>;
  }
  if (t.name.startsWith("--border-radius-")) {
    return <span className="theme_story__radius" style={{ borderRadius: `var(${t.name})` }} />;
  }
  return null;
};

const ScalesStory: React.FC = () => (
  <Page>
    {(values) => {
      const scales = values.filter((t) => t.scopes.has("theme") && !isColor(t.value));
      const groups = groupBy(scales, (t) =>
        t.name.replace(/^--([a-z]+(?:-(?:size|weight|height|radius|index))?).*$/, "$1")
      );
      return (
        <>
          {Object.entries(groups).map(([name, tokens]) => (
            <Section key={name} title={name} count={tokens.length}>
              <table className="theme_story__table">
                <tbody>
                  {tokens.map((t) => (
                    <tr key={t.name}>
                      <td>
                        <code className="theme_story__name">{t.name}</code>
                      </td>
                      <td>
                        <code className="theme_story__value">{t.value}</code>
                      </td>
                      <td>{scaleSample(t)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </Section>
          ))}
        </>
      );
    }}
  </Page>
);

const stories: Story[] = [
  {
    name: "Palette",
    component: () => <PaletteStory />,
  },
  {
    name: "Themed",
    component: () => <ThemedStory />,
  },
  {
    name: "Elevation",
    component: () => <ElevationStory />,
  },
  {
    name: "Scales",
    component: () => <ScalesStory />,
  },
];

export default stories;
