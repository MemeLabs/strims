import clsx from "clsx";
import Prism from "prismjs";
import React, { KeyboardEvent, useCallback, useMemo, useState } from "react";
import { FunctionComponent } from "react";
import { Editor, Node, Range, Text, Transforms, createEditor } from "slate";
import { Editable, Slate, useFocused, useSelected, withReact } from "slate-react";
import urlRegex from "url-regex-safe";

import { emotes, modifiers } from "./test-emotes";
import users from "./test-users";

const tags = ["nsfw", "weeb", "nsfl", "loud"];

interface ComposerProps {}

const Composer: FunctionComponent<ComposerProps> = (props) => {
  const [target, setTarget] = useState<Range | undefined>();
  const [index, setIndex] = useState(0);
  const [search, setSearch] = useState("");
  const editor = useMemo(() => withReact(createEditor()), []);
  const [value, setValue] = useState([
    {
      type: "paragraph",
      children: [{ text: "|| test \\|| test" }],
    },
  ] as Node[]);

  const emoteNames = useMemo(() => emotes.map(({ name }) => name), [emotes]);
  const nicks = useMemo(() => users.map(({ nick }) => nick), [users]);
  const matches = useMemo(() => {
    if (!search) {
      return [];
    }

    // TODO: prioritize recently used
    const query = search.toLowerCase();
    const match = (t: string) => t.toLowerCase().startsWith(query);
    return [
      ...emoteNames.filter(match),
      ...nicks.filter(match),
      ...tags.filter(match),
      ...modifiers.filter(match),
    ].slice(0, 10);
  }, [emoteNames, search]);

  const renderLeaf = useCallback((props) => <Leaf {...props} />, []);

  const language = useMemo(() => {
    const nestableEntities = {
      code: {
        pattern: /`(\\`|[^`])*(`|$)/,
        greedy: true,
      },
      emote: new RegExp(`(${emoteNames.join("|")})(:(${modifiers.join("|")}))*`),
      nick: new RegExp(nicks.join("|")),
      tag: new RegExp(tags.join("|")),
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
  }, [emoteNames, modifiers, nicks, tags]);

  const decorate = useCallback(([node, path]) => {
    const ranges = [];

    if (!Text.isText(node)) {
      return ranges;
    }

    const appendRanges = (token: string | Prism.Token, start: number = 0) => {
      if (typeof token === "string") {
        return token.length;
      }

      const content = Array.isArray(token.content) ? token.content : [token.content];
      const length: number = content.reduce((l: number, t) => l + appendRanges(t, start + l), 0);

      ranges.push({
        [token.type]: true,
        anchor: { path, offset: start },
        focus: { path, offset: start + length },
      });

      return length;
    };

    const tokens = Prism.tokenize(node.text, language);
    tokens.reduce((l: number, t) => l + appendRanges(t, l), 0);

    return ranges;
  }, []);

  const onChange = (newValue) => {
    setValue(newValue);
    const { selection } = editor;

    if (selection && Range.isCollapsed(selection)) {
      const [cursor] = Range.edges(selection);
      const wordStart = Editor.before(editor, cursor, { unit: "word" });
      const wordEnd = wordStart && Editor.after(editor, wordStart, { unit: "word" });
      const wordRange = wordEnd && Editor.range(editor, wordStart, wordEnd);
      const text = wordRange && Editor.string(editor, wordRange);

      // TODO: context aware autocomplete for modifiers
      const prevNode = wordRange && Editor.previous(editor, { at: wordRange });

      console.log({ text, prevNode });

      if (text) {
        setSearch(text);
        setTarget(wordRange);
        return;
      }
    }

    setTarget(null);
  };

  const onKeyDown = useCallback(
    (event: KeyboardEvent<HTMLDivElement>) => {
      if (!target) {
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
          Transforms.select(editor, target);
          Transforms.insertText(editor, matches[index]);
          Transforms.move(editor);
          setTarget(null);
          break;
        }
        case "Escape": {
          event.preventDefault();
          setTarget(null);
          break;
        }
      }
    },
    [index, search, target]
  );

  // console.log({ target, search, index });

  return (
    <div className="chat__composer">
      {!!matches.length && <div className="chat__composer__autocomplete">{matches.join(", ")}</div>}
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
