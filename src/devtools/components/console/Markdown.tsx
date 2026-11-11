// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

/** `text`'s inline code, bold and italics as elements. */
const inline = (text: string): React.ReactNode[] => {
  const out: React.ReactNode[] = [];
  const re = /`([^`]+)`|\*\*([^*]+)\*\*|\*([^*\s][^*]*)\*/g;
  let at = 0;
  for (const m of text.matchAll(re)) {
    if (m.index > at) out.push(text.slice(at, m.index));
    if (m[1] !== undefined) out.push(<code key={m.index}>{m[1]}</code>);
    else if (m[2] !== undefined) out.push(<strong key={m.index}>{m[2]}</strong>);
    else out.push(<em key={m.index}>{m[3]}</em>);
    at = m.index + m[0].length;
  }
  if (at < text.length) out.push(text.slice(at));
  return out;
};

const cells = (line: string): string[] =>
  line
    .trim()
    .replace(/^\||\|$/g, "")
    .split("|")
    .map((c) => c.trim());

const LIST = /^\s*(?:[-*]|\d+\.)\s+/;

/**
 * The markdown Claude answers in, as elements (never as HTML): headings, paragraphs, lists, pipe
 * tables, fenced code, and inline code, bold and italics.
 */
const Markdown: React.FC<{ text: string }> = ({ text }) => {
  const lines = text.split("\n");
  const blocks: React.ReactNode[] = [];
  let i = 0;
  while (i < lines.length) {
    const line = lines[i];
    if (/^\s*```/.test(line)) {
      const body: string[] = [];
      for (i++; i < lines.length && !/^\s*```/.test(lines[i]); i++) body.push(lines[i]);
      i++;
      blocks.push(<pre key={i}>{body.join("\n")}</pre>);
    } else if (/^\s*\|/.test(line)) {
      const rows: string[][] = [];
      for (; i < lines.length && /^\s*\|/.test(lines[i]); i++) {
        if (!/^\s*\|[\s:|-]+\|?\s*$/.test(lines[i])) rows.push(cells(lines[i]));
      }
      const [head, ...body] = rows;
      blocks.push(
        <table key={i} className="console__table">
          <thead>
            <tr>
              {head.map((c, j) => (
                <th key={j}>{inline(c)}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {body.map((r, k) => (
              <tr key={k}>
                {r.map((c, j) => (
                  <td key={j}>{inline(c)}</td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      );
    } else if (LIST.test(line)) {
      const items: string[] = [];
      const ordered = /^\s*\d+\./.test(line);
      for (; i < lines.length && (LIST.test(lines[i]) || /^\s{2,}\S/.test(lines[i])); i++) {
        if (LIST.test(lines[i])) items.push(lines[i].replace(LIST, ""));
        else items[items.length - 1] += " " + lines[i].trim();
      }
      const children = items.map((t, k) => <li key={k}>{inline(t)}</li>);
      blocks.push(ordered ? <ol key={i}>{children}</ol> : <ul key={i}>{children}</ul>);
    } else if (/^#{1,6}\s/.test(line)) {
      blocks.push(<h4 key={i}>{inline(line.replace(/^#+\s*/, ""))}</h4>);
      i++;
    } else if (line.trim() === "") {
      i++;
    } else {
      const para: string[] = [];
      for (
        ;
        i < lines.length &&
        lines[i].trim() !== "" &&
        !/^\s*(```|\||#{1,6}\s)/.test(lines[i]) &&
        !LIST.test(lines[i]);
        i++
      )
        para.push(lines[i]);
      blocks.push(<p key={i}>{inline(para.join(" "))}</p>);
    }
  }
  return <div className="console__markdown">{blocks}</div>;
};

export default Markdown;
