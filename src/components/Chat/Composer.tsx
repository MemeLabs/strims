import clsx from "clsx";
import filterObj from "filter-obj";
import Prism from "prismjs";
import React, { KeyboardEvent, useCallback, useMemo, useState } from "react";
import { FunctionComponent } from "react";
import { Editor, Node, NodeEntry, Path, Point, Range, Text, Transforms, createEditor } from "slate";
import { Editable, Slate, useFocused, useSelected, withReact } from "slate-react";
import urlRegex from "url-regex-safe";

import { emotes, modifiers } from "./test-emotes";
import users from "./test-users";

const tags = ["nsfw", "weeb", "nsfl", "loud"];
const commands = [
  "help",
  "emotes",
  "me",
  "message",
  "msg",
  "ignore",
  "unignore",
  "highlight",
  "unhighlight",
  "maxlines",
  "mute",
  "unmute",
  "subonly",
  "ban",
  "unban",
  "timestampformat",
  "tag",
  "untag",
  "exit",
  "hideemote",
  "unhideemote",
];

type SearchState =
  | { enabled: false }
  | {
      enabled: true;
      debounceDelay: number;
      queryMode: "substring" | "prefix";
      sources: string[][];
      query: string;
      target: Range;
      modifierContext: string | undefined;
      modifierTarget: Range;
    };

interface ComposerProps {}

const Composer: FunctionComponent<ComposerProps> = (props) => {
  const [index, setIndex] = useState(0);
  const [search, setSearch] = useState<SearchState>({ enabled: false });
  const editor = useMemo(() => withReact(createEditor()), []);
  const [value, setValue] = useState([
    {
      type: "paragraph",
      children: [{ text: "" }],
    },
  ] as Node[]);

  const emoteNames = useMemo(() => emotes.map(({ name }) => name), [emotes]);
  const nicks = useMemo(() => users.map(({ nick }) => nick), [users]);
  const matches = useMemo(() => {
    if (!search.enabled) {
      return [];
    }

    console.log(search);
    const query = search.query.toLowerCase();
    const test =
      search.queryMode === "prefix"
        ? (term: string) => term.toLowerCase().startsWith(query)
        : (term: string) => term.toLowerCase().indexOf(query) !== -1;

    return search.sources
      .map((s) => s.filter(test))
      .flat()
      .slice(0, 10);
  }, [search]);

  const renderLeaf = useCallback((props) => <Leaf {...props} />, []);

  const grammar = useMemo(() => getGrammar(emoteNames, modifiers, nicks, tags), [
    emoteNames,
    modifiers,
    nicks,
    tags,
  ]);

  const decorate = useCallback(
    ([node, path]: NodeEntry<Node>) =>
      Text.isText(node) ? getRanges(node.text, path, grammar) : [],
    [grammar]
  );

  const onChange = (newValue) => {
    setValue(newValue);

    const { selection } = editor;
    const [node, path] = Editor.node(editor, selection);
    if (!selection || !Range.isCollapsed(selection) || !Text.isText(node)) {
      setSearch({ enabled: false });
      return;
    }

    const { text } = node;
    const { offset } = Range.start(selection);
    const [, contiguousContext, prefix, delim, punct, queryStart] =
      /(\w(?=::|@))?((\s*(:|@|^\/)?)?(\w*))$/.exec(text.substring(0, offset)) || [];
    const [queryEnd] = /^\w+/.exec(text.substring(offset)) || [];
    const query = (queryStart || "") + (queryEnd || "");

    const targetStart = { path, offset: offset - (queryStart?.length || 0) - (punct?.length || 0) };
    const targetEnd = { path, offset: offset + (queryEnd?.length || 0) };
    const target = Editor.range(editor, targetStart, targetEnd);

    const contextRanges = getRanges(text, path, filterObj(grammar, ["code", "emote", "url"]));

    const contextEnd = delim && { path, offset: offset - prefix.length };
    const modifierContextRange =
      contextEnd && contextRanges.find((r) => r.emote && Range.includes(r, contextEnd));
    const modifierContext = modifierContextRange && Editor.string(editor, modifierContextRange);
    const modifierTarget = modifierContextRange && Editor.range(editor, contextEnd, targetEnd);

    const invalidContext =
      (contiguousContext && !modifierContext) ||
      contextRanges.some((r) => (r.code || r.url) && Range.includes(r, selection));

    const sources = [];
    if (punct === ":") {
      if (modifierContext) {
        sources.push(modifiers);
      }
      if (!contiguousContext) {
        sources.push(emoteNames);
      }
    } else if (punct === "@") {
      sources.push(nicks);
    } else if (punct === "/") {
      sources.push(commands);
    } else {
      sources.push(emoteNames, nicks, tags);
    }

    setSearch({
      enabled: !invalidContext && !!(punct || query) && sources.length > 0,
      debounceDelay: punct ? 0 : 100,
      queryMode: punct ? "substring" : "prefix",
      sources,
      query,
      target,
      modifierContext,
      modifierTarget,
    });
  };

  const onKeyDown = useCallback(
    (event: KeyboardEvent<HTMLDivElement>) => {
      if (!search.enabled) {
        return;
      }

      switch (event.key) {
        case "ArrowDown": {
          event.preventDefault();
          const prevIndex = index >= matches.length - 1 ? 0 : index + 1;
          setIndex(prevIndex);
          break;
        }
        case "ArrowUp": {
          event.preventDefault();
          const nextIndex = index <= 0 ? matches.length - 1 : index - 1;
          setIndex(nextIndex);
          break;
        }
        case "Tab":
        case "Enter": {
          // TODO: tab multiple times to select autocomplete
          event.preventDefault();
          Transforms.select(editor, search.target);
          Transforms.insertText(editor, matches[index]);
          Transforms.move(editor);
          setSearch({ enabled: false });
          break;
        }
        case "Escape": {
          event.preventDefault();
          setSearch({ enabled: false });
          break;
        }
      }
    },
    [index, search]
  );

  // console.log({ target, search, index });

  return (
    <div className="chat__composer">
      {search.enabled && matches.length > 0 && (
        <div className="chat__composer__autocomplete">
          <ul>
            {matches.map((m, i) => (
              <li key={i}>{m}</li>
            ))}
          </ul>
        </div>
      )}
      <Slate editor={editor} value={value} onChange={onChange}>
        <Editable
          decorate={decorate}
          onKeyDown={onKeyDown}
          placeholder="Write a message..."
          renderLeaf={renderLeaf}
        />
      </Slate>
    </div>
  );
};

export default Composer;

const getGrammar = (emotes: string[], modifiers: string[], nicks: string[], tags: string[]) => {
  const nestableEntities = {
    code: {
      pattern: /`(\\`|[^`])*(`|$)/,
      greedy: true,
    },
    emote: {
      pattern: new RegExp(`(\\W|^)((${emotes.join("|")})(:(${modifiers.join("|")}))*)(?=\\W|$)`),
      lookbehind: true,
    },
    nick: {
      pattern: new RegExp(`(\\W|^)(${nicks.join("|")})(?=\\W|$)`),
      lookbehind: true,
    },
    tag: {
      pattern: new RegExp(`(\\W|^)(${tags.join("|")})(?=\\W|$)`),
      lookbehind: true,
    },
    url: urlRegex(),
  };
  const entities = {
    spoiler: {
      pattern: /\|\|(\\\||\|(?!\|)|[^|])*(\|\||$)/,
      inside: nestableEntities,
    },
    ...nestableEntities,
  };
  return {
    greentext: {
      pattern: /^>.*/,
      greedy: true,
      inside: entities,
    },
    self: {
      pattern: /^\/me .*/,
      greedy: true,
      inside: entities,
    },
    ...entities,
  };
};

const getRanges = (text: string, path: Path, grammar: Prism.Grammar) => {
  const ranges: Range[] = [];

  const appendRanges = (tokens: (string | Prism.Token)[], start: number = 0) =>
    tokens.reduce((offset, token) => {
      if (typeof token !== "string") {
        ranges.push({
          [token.type]: true,
          anchor: { path, offset: offset },
          focus: { path, offset: offset + token.length },
        });
        appendRanges(Array.isArray(token.content) ? token.content : [token.content], offset);
      }
      return offset + token.length;
    }, start);
  appendRanges(Prism.tokenize(text, grammar));

  return ranges;
};

const Leaf = ({ attributes, children, leaf }) => {
  return (
    <span
      {...attributes}
      className={clsx({
        "chat__composer__span--code": leaf.code,
        "chat__composer__span--spoiler": leaf.spoiler,
        "chat__composer__span--url": leaf.url,
        "chat__composer__span--emote": leaf.emote,
        "chat__composer__span--tag": leaf.tag,
        "chat__composer__span--nick": leaf.nick,
        "chat__composer__span--self": leaf.self,
        "chat__composer__span--greentext": leaf.greentext,
      })}
      spellCheck={!leaf.emote && !leaf.tag && !leaf.nick}
    >
      {children}
    </span>
  );
};
