// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Composer.scss";

import { useDrag } from "@use-gesture/react";
import clsx from "clsx";
import filterObj from "filter-obj";
import { escapeRegExp } from "lodash";
import Prism from "prismjs";
import React, {
  KeyboardEvent,
  MouseEventHandler,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import {
  Descendant,
  Editor,
  Element,
  Node,
  NodeEntry,
  Path,
  Range,
  Text,
  Transforms,
  createEditor,
} from "slate";
import { withHistory } from "slate-history";
import { Editable, RenderLeafProps, Slate, withReact } from "slate-react";
import { Key } from "ts-key-enum";
import urlRegex from "url-regex-safe";

import { EmojiCategory } from "../../apis/strims/chat/v1/chat";
import { useChat } from "../../contexts/Chat";
import Emote from "./Emote";

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
  "spoiler",
];

const initialValue: Descendant[] = [
  {
    type: "paragraph",
    children: [{ text: "" }],
  },
];

interface SelectedMatch {
  index: number;
  entry: SearchSourceEntry;
}

type Match = [SearchSource[], SearchSourceEntry[]];

const defaultSelectedMatch: SelectedMatch = { index: 0, entry: null };
const defaultMatch: Match = [[], []];

interface ComposerProps {
  onMessage: (message: string) => void;
  emotes: string[];
  modifiers: string[];
  nicks: string[];
  tags: string[];
  maxAutoCompleteResults?: number;
}

const Composer: React.FC<ComposerProps> = ({
  onMessage,
  emotes,
  modifiers,
  nicks,
  tags,
  maxAutoCompleteResults = 10,
}) => {
  const { t } = useTranslation();
  const [{ uiConfig, emoji }] = useChat();

  const [[matchSources, matchEntries], setMatch] = useState<Match>(defaultMatch);
  const [selectedMatch, setSelectedMatch] = useState<SelectedMatch>(defaultSelectedMatch);
  const [currentSearch, setSearch] = useState<SearchState | null>(null);
  const [lastSearch, setLastSearch] = useState<SearchState | null>(null);
  const search = lastSearch || currentSearch;

  const editor = useMemo(() => withReact(withNoLineBreaks(withHistory(createEditor()))), []);
  const [value, setValue] = useState<Descendant[]>(initialValue);

  const searchSources = useSearchSources(nicks, tags, commands, emotes, modifiers, emoji);

  useEffect(() => {
    if (!search) {
      setMatch(defaultMatch);
      setSelectedMatch(defaultSelectedMatch);
      return;
    }
    if (lastSearch) {
      return;
    }

    const query = search.query.toLowerCase();
    const test = {
      "prefix": ({ index }: SearchSourceEntry) => index.startsWith(query),
      "substring": ({ index }: SearchSourceEntry) => index.indexOf(query) !== -1,
    }[search.queryMode];

    let count = 0;
    const truncate = (entries: SearchSourceEntry[]) => {
      const res = entries.slice(0, maxAutoCompleteResults - count);
      count += res.length;
      return res;
    };

    const matches = search.sources
      .map((s) => ({
        ...s,
        entries: truncate(s.entries.filter(test)),
      }))
      .filter(({ entries }) => entries.length > 0);
    const entries = matches.map(({ entries }) => entries).flat();

    setMatch([matches, entries]);
    setSelectedMatch({ index: 0, entry: entries[0] });
  }, [search]);

  const renderLeaf = useCallback((props: RenderLeafProps) => <Leaf {...props} />, []);

  const grammar = useMemo(
    () => getGrammar(emotes, modifiers, emoji, nicks, tags),
    [emotes, modifiers, emoji, nicks, tags]
  );

  const decorate = useCallback(
    ([node, path]: NodeEntry<Node>) =>
      Text.isText(node) ? getRanges(node.text, path, grammar) : [],
    [grammar]
  );

  const onChange = useCallback(
    (newValue: Descendant[]) => {
      setValue(newValue);
      setSearch(getSearchState(editor, grammar, searchSources));
    },
    [editor, grammar, searchSources]
  );

  const emitMessage = () => {
    const body = ComposerEditor.text(editor).trim();
    if (body) {
      onMessage(body);
      ComposerEditor.clear(editor);
    }
  };

  const onKeyDown = useCallback(
    (event: KeyboardEvent<HTMLDivElement>) => {
      if (event.key === Key.Enter) {
        event.preventDefault();
        emitMessage();
      }

      if (!search) {
        return;
      }

      const getSelectedMatch = (i: number) => ({
        index: i,
        entry: matchEntries[i % matchEntries.length],
      });

      switch (event.key) {
        case Key.ArrowDown: {
          event.preventDefault();
          setSelectedMatch(({ index }) => getSelectedMatch(index + 1));
          return;
        }
        case Key.ArrowUp: {
          event.preventDefault();
          setSelectedMatch(({ index }) => getSelectedMatch(index - 1));
          return;
        }
        case Key.Tab: {
          event.preventDefault();

          if (!selectedMatch.entry) {
            return;
          }

          const target = insertAutocompleteEntry(selectedMatch.entry);
          setLastSearch({
            ...search,
            target,
            lastEntry: selectedMatch.entry,
          });
          setSelectedMatch(({ index }) => getSelectedMatch(index + 1));
          return;
        }
        case Key.Escape: {
          event.preventDefault();
          setSearch(null);
          break;
        }
      }

      setLastSearch(null);
      setSelectedMatch(defaultSelectedMatch);
    },
    [matchEntries, selectedMatch, search, currentSearch]
  );

  const insertAutocompleteEntry = (entry: SearchSourceEntry): Range => {
    const prefix = entry.type !== "modifier" ? search.prefix : "";
    const suffix = search.suffixSpace ? " " : "";
    const substitution = prefix + (entry.substitution || entry.value) + suffix;

    Transforms.select(editor, search.target);
    Transforms.insertText(editor, substitution);
    Transforms.move(editor);

    const anchor = search.target.anchor;
    const targetFocus = {
      path: anchor.path,
      offset: anchor.offset + substitution.length,
    };
    return Editor.range(editor, anchor, targetFocus);
  };

  const handleAutocompleteSelect = (entry: SearchSourceEntry): void => {
    insertAutocompleteEntry(entry);
    // setLastSearch(search);
    setLastSearch(null);
    setSearch(null);
    setSelectedMatch(defaultSelectedMatch);
  };

  const showSuggestions =
    uiConfig.autocompleteHelper && search && matchEntries.length > (lastSearch ? 1 : 0);

  // console.log(selectedMatch);
  // console.log({
  //   matchSources,
  //   matchEntries,
  //   selectedMatch,
  //   search,
  // });

  const ref = useRef<HTMLDivElement>();

  useDrag(
    ({ dragging, movement: [mx, my] }) => {
      if (!dragging && my < -50 && Math.abs(mx) < 50) {
        emitMessage();
      }
    },
    {
      axis: "y",
      target: ref,
      boundToParent: false,
      eventOptions: {
        capture: true,
      },
    }
  );

  return (
    <div className="chat_composer">
      {showSuggestions && (
        <div className="chat_composer__autocomplete">
          <div className="chat_composer__autocomplete__list">
            {matchSources.map((m, i) => (
              <AutocompleteGroup
                {...m}
                selectedMatch={selectedMatch}
                onSelect={handleAutocompleteSelect}
                key={i}
              />
            ))}
          </div>
        </div>
      )}
      <div className="chat_composer__editor" ref={ref}>
        <Slate editor={editor} value={value} onChange={onChange}>
          <Editable
            decorate={decorate}
            onKeyDown={onKeyDown}
            placeholder={t("chat.composer.Write a message")}
            renderLeaf={renderLeaf}
          />
        </Slate>
      </div>
    </div>
  );
};

interface AutocompleteGroupProps {
  label: string;
  entries: SearchSourceEntry[];
  selectedMatch: SelectedMatch;
  onSelect: (entry: SearchSourceEntry) => void;
}

const AutocompleteGroup: React.FC<AutocompleteGroupProps> = ({
  label,
  entries,
  selectedMatch,
  onSelect,
}) => {
  const clickHandler =
    (entry: SearchSourceEntry): MouseEventHandler =>
    (e) => {
      e.preventDefault();
      onSelect(entry);
    };

  return (
    <>
      <div className="chat_composer__autocomplete__label">{label}</div>
      {entries.map((e, i) => (
        <AutocompleteGroupItem
          entry={e}
          selectedMatch={selectedMatch}
          onClick={clickHandler(e)}
          key={i}
        />
      ))}
    </>
  );
};

interface AutocompleteGroupItemProps {
  entry: SearchSourceEntry;
  selectedMatch: SelectedMatch;
  onClick: MouseEventHandler;
}

const AutocompleteGroupItem: React.FC<AutocompleteGroupItemProps> = ({
  entry,
  selectedMatch,
  onClick,
}) => {
  const [{ uiConfig }] = useChat();

  let preview: React.ReactNode;
  if (uiConfig.autocompleteEmotePreview) {
    if (entry.type === "emote") {
      preview = (
        <span className="chat_composer__autocomplete__item__emote">
          <Emote name={entry.value} shouldAnimateForever />
        </span>
      );
    } else if (entry.type === "emoji") {
      preview = (
        <span className="chat_composer__autocomplete__item__emoji">{entry.substitution}</span>
      );
    }
  }

  return (
    <div
      className={clsx(
        "chat_composer__autocomplete__item",
        `chat_composer__autocomplete__item--${entry.type}`,
        {
          "chat_composer__autocomplete__item--selected": entry === selectedMatch.entry,
        }
      )}
      onClick={onClick}
      onMouseDown={(e) => e.preventDefault()}
    >
      {preview}
      <span className="chat_composer__autocomplete__item__label">{entry.value}</span>
    </div>
  );
};

const Leaf: React.FC<RenderLeafProps> = ({ attributes, children, leaf }) => {
  if (leaf.emote) {
    const [{ uiConfig }] = useChat();
    const [name, ...modifiers] = leaf.text.split(":");

    return (
      <Emote
        {...attributes}
        name={name}
        modifiers={modifiers}
        shouldAnimateForever={uiConfig.animateForever}
        shouldShowModifiers={uiConfig.emoteModifiers}
        compactSpacing={uiConfig.compactEmoteSpacing}
      >
        {children}
      </Emote>
    );
  }

  return (
    <span
      {...attributes}
      className={clsx({
        "chat_composer__span--code": leaf.code,
        "chat_composer__span--spoiler": leaf.spoiler,
        "chat_composer__span--url": leaf.url,
        "chat_composer__span--emote": leaf.emote,
        "chat_composer__span--emoji": leaf.emoji,
        "chat_composer__span--tag": leaf.tag,
        "chat_composer__span--nick": leaf.nick,
        "chat_composer__span--self": leaf.self,
        "chat_composer__span--greentext": leaf.greentext,
      })}
      spellCheck={!leaf.emote && !leaf.tag && !leaf.nick}
    >
      {children}
    </span>
  );
};

export default Composer;

const noopPattern = /_^/;

const getGrammar = (
  emotes: string[],
  modifiers: string[],
  emoji: EmojiCategory[],
  nicks: string[],
  tags: string[]
) => {
  const nestableEntities = {
    code: {
      pattern: /`(\\`|[^`])*(`|$)/,
      greedy: true,
    },
    emote: {
      pattern: noopPattern,
      lookbehind: true,
    },
    emoji: {
      pattern: noopPattern,
    },
    nick: {
      pattern: noopPattern,
      lookbehind: true,
    },
    tag: {
      pattern: noopPattern,
      lookbehind: true,
    },
    url: urlRegex(),
  };

  if (emotes.length !== 0 && modifiers.length !== 0) {
    nestableEntities.emote.pattern = new RegExp(
      `(\\W|^)((${emotes.join("|")})(:(${modifiers.join("|")}))*)(?=\\W|$)`
    );
  } else if (emotes.length !== 0) {
    nestableEntities.emote.pattern = new RegExp(`(\\W|^)(${emotes.join("|")})(?=\\W|$)`);
  }
  if (emoji.length !== 0) {
    const glyphs: string[] = [];
    for (const category of emoji) {
      glyphs.push(...category.emoji.map(({ glyph }) => escapeRegExp(glyph)));
    }
    glyphs.sort((a, b) => (a > b ? -1 : a < b ? 1 : 0));
    nestableEntities.emoji.pattern = new RegExp(glyphs.join("|"));
  }
  if (nicks.length !== 0) {
    nestableEntities.nick.pattern = new RegExp(`(\\W|^)(${nicks.join("|")})(?=\\W|$)`);
  }
  if (tags.length !== 0) {
    nestableEntities.tag.pattern = new RegExp(`(\\W|^)(${tags.join("|")})(?=\\W|$)`);
  }

  const entities = {
    spoiler: {
      pattern: /(^\/spoiler|\|\|)(\\\||\|(?!\|)|[^|])*(\|\||$)/,
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

type Grammar = ReturnType<typeof getGrammar>;

const getRanges = (text: string, path: Path, grammar: Prism.Grammar) => {
  const ranges: Range[] = [];

  const appendRanges = (tokens: (string | Prism.Token)[], start: number = 0) => {
    let offset = start;
    for (const token of tokens) {
      if (typeof token !== "string") {
        ranges.push({
          [token.type]: true,
          anchor: { path, offset: offset },
          focus: { path, offset: offset + token.length },
        });
        appendRanges(Array.isArray(token.content) ? token.content : [token.content], offset);
      }
      offset += token.length;
    }
  };
  appendRanges(Prism.tokenize(text, grammar));

  return ranges;
};

interface SearchSourceEntry {
  type: "nick" | "tag" | "command" | "emote" | "modifier" | "emoji";
  value: string;
  substitution?: string;
  index: string;
}

interface SearchSource {
  label: string;
  entries: SearchSourceEntry[];
}

interface SearchSources {
  nicks: SearchSource;
  tags: SearchSource;
  commands: SearchSource;
  emotes: SearchSource;
  modifiers: SearchSource;
  emoji: SearchSource;
}

const useSearchSources = (
  nicks: string[],
  tags: string[],
  commands: string[],
  emotes: string[],
  modifiers: string[],
  emoji: EmojiCategory[]
): SearchSources => {
  const { t } = useTranslation();
  const sources: SearchSources = {
    nicks: useMemo(
      () => ({
        label: t("chat.composer.members"),
        entries: nicks.map((v) => ({
          type: "nick",
          value: v,
          index: v.toLowerCase(),
        })),
      }),
      [nicks]
    ),
    tags: useMemo(
      () => ({
        label: t("chat.composer.tags"),
        entries: tags.map((v) => ({
          type: "tag",
          value: v,
          index: v.toLowerCase(),
        })),
      }),
      [tags]
    ),
    commands: useMemo(
      () => ({
        label: t("chat.composer.commands"),
        entries: commands.map((v) => ({
          type: "command",
          value: v,
          substitution: "/" + v,
          index: v.toLowerCase(),
        })),
      }),
      [commands]
    ),
    emotes: useMemo(
      () => ({
        label: t("chat.composer.emotes"),
        entries: emotes.map((v) => ({
          type: "emote",
          value: v,
          index: v.toLowerCase(),
        })),
      }),
      [emotes]
    ),
    modifiers: useMemo(
      () => ({
        label: t("chat.composer.modifiers"),
        entries: modifiers.map((v) => ({
          type: "modifier",
          value: v,
          substitution: ":" + v,
          index: v.toLowerCase(),
        })),
      }),
      [modifiers]
    ),
    emoji: useMemo(() => {
      const source: SearchSource = {
        label: t("chat.composer.emoji"),
        entries: [],
      };
      for (const category of emoji) {
        for (const { glyph, description } of category.emoji) {
          source.entries.push({
            type: "emoji",
            value: description,
            substitution: glyph,
            index: description,
          });
        }
      }
      return source;
    }, [emoji]),
  };
  return useMemo(() => sources, [nicks, tags, commands, emotes, modifiers, emoji]);
};

interface SearchState {
  debounceDelay: number;
  queryMode: "substring" | "prefix";
  sources: SearchSource[];
  query: string;
  target: Range;
  prefix: string;
  suffixSpace: boolean;
  lastEntry?: SearchSourceEntry;
}

const getSearchState = (
  editor: Editor,
  grammar: Grammar,
  searchSources: SearchSources
): SearchState => {
  const { selection } = editor;
  if (!selection || !Range.isCollapsed(selection)) {
    return null;
  }

  const [node, path] = Editor.node(editor, selection);
  if (!Text.isText(node)) {
    return null;
  }

  const { text } = node;
  const { offset } = Range.start(selection);
  const [, contiguousContext, delta, prefix, punct, queryStart, suffixStart] =
    /(\w(?=:|@))?((\s*)(:|@|^\/)?(\w*)(\s*))$/.exec(text.substring(0, offset)) || [];
  const [, suffixEnd, queryEnd] = /^(\s*)(\w*)/.exec(text.substring(offset)) || [""];
  const hasSuffix = suffixStart || suffixEnd;
  const query = queryStart + (hasSuffix ? "" : queryEnd);

  const targetStart = { path, offset: offset - delta.length };
  const targetEnd = { path, offset: offset + (hasSuffix ? -suffixStart.length : queryEnd.length) };
  const target = Editor.range(editor, targetStart, targetEnd);

  const entityRanges = getRanges(text, path, filterObj(grammar, ["code", "emote", "url"]));

  const contextEnd = (prefix || punct) && { path, offset: offset - delta.length };
  const emoteContext =
    contextEnd && entityRanges.some((r) => r.emote && Range.includes(r, contextEnd));

  const invalidContext =
    (contiguousContext && !emoteContext) ||
    entityRanges.some((r) => (r.code || r.url) && Range.includes(r, selection));

  const sources: SearchSource[] = [];
  if (punct === ":") {
    if (emoteContext) {
      sources.push(searchSources.modifiers);
    }
    if (!contiguousContext) {
      sources.push(searchSources.emotes, searchSources.emoji);
    }
  } else if (punct === "@") {
    sources.push(searchSources.nicks);
  } else if (punct === "/") {
    sources.push(searchSources.commands);
  } else {
    if (emoteContext) {
      sources.push(searchSources.modifiers);
    }
    sources.push(searchSources.emotes, searchSources.nicks, searchSources.tags);
  }

  // console.log({
  //   text,
  //   offset,
  //   contiguousContext,
  //   delta,
  //   prefix,
  //   punct,
  //   queryStart,
  //   suffixStart,
  //   suffixEnd,
  //   queryEnd,
  //   hasSuffix,
  //   query,
  //   targetStart,
  //   targetEnd,
  //   target,
  //   entityRanges,
  //   contextEnd,
  //   invalidContext,
  //   sources,
  // });

  if (invalidContext || !(punct || query) || sources.length === 0) {
    return null;
  }

  return {
    debounceDelay: punct ? 0 : 100,
    queryMode: punct ? "substring" : "prefix",
    sources,
    query,
    target,
    prefix,
    suffixSpace: !hasSuffix,
  };
};

const ComposerEditor = {
  clear: (editor: Editor) => {
    const [[node, path]] = Editor.nodes(editor, { match: (n) => Text.isText(n) });
    if (!Text.isText(node)) {
      return;
    }

    Transforms.select(
      editor,
      Editor.range(
        editor,
        {
          offset: 0,
          path,
        },
        {
          offset: node.text.length,
          path,
        }
      )
    );
    Transforms.delete(editor);
    Transforms.move(editor);
  },

  text: (editor: Editor) => {
    const [[node]] = Editor.nodes(editor, { match: (n) => Text.isText(n) });
    return Text.isText(node) ? node.text : "";
  },
};

const withNoLineBreaks = (editor: Editor) => {
  const { normalizeNode } = editor;

  editor.normalizeNode = (entry) => {
    const [node] = entry;

    if (Editor.isEditor(node) && node.children.length > 1) {
      Transforms.mergeNodes(editor, {
        match: (node) => Element.isElement(node) && node.type === "paragraph",
      });
    }

    normalizeNode(entry);
  };

  return editor;
};
