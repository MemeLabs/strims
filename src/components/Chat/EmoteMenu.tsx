// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./EmoteMenu.scss";

import clsx from "clsx";
import { CompactEmoji } from "emojibase";
import { escapeRegExp } from "lodash";
import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Scrollbars } from "react-custom-scrollbars-2";
import { BiSearch } from "react-icons/bi";
import {
  FaBasketballBall,
  FaClock,
  FaDiceD6,
  FaFlag,
  FaLaugh,
  FaPeace,
  FaPlaneDeparture,
  FaQuestionCircle,
  FaTheaterMasks,
  FaTree,
  FaWalking,
} from "react-icons/fa";
import { MdClose, MdFastfood } from "react-icons/md";
import { useDebounce } from "react-use";

import * as chatv1 from "../../apis/strims/chat/v1/chat";
import { useChat, useRoom } from "../../contexts/Chat";
import useSize from "../../hooks/useSize";
import { useStableCallback } from "../../hooks/useStableCallback";
import balanceRows from "../../lib/balanceRows";
import Dropdown from "../Dropdown";
import Emoji from "./Emoji";
import Emote from "./Emote";
import { computeEmoteWidth } from "./StyleSheet";

const DOUBLE_CLICK_TIMEOUT = 200;

// needs to be kept in sync with stylesheets
const EMOTE_MARGIN = 3;
const VIEWPORT_MARGIN = 10;
const EMOTE_GAP = 0;

interface EmoteMenuProps {
  onSelect: (v: string, send: boolean) => void;
  onClose: () => void;
}

const EmoteMenu: React.FC<EmoteMenuProps> = ({ onSelect, onClose }) => {
  const [{ uiConfig, emoji }, { mergeUIConfig }] = useChat();
  const [{ liveEmotes }] = useRoom();

  const [searchQuery, setSearchQuery] = useState("");
  const [debouncedSearchQuery, setDebouncedSearchQuery] = React.useState("");
  useDebounce(() => setDebouncedSearchQuery(searchQuery), 100, [searchQuery]);

  const categories = useMemo(() => {
    const pattern = debouncedSearchQuery
      ? new RegExp(escapeRegExp(debouncedSearchQuery), "i")
      : null;

    const categories: Category[] = [];

    const matchedEmotes = liveEmotes.filter(({ name }) => !pattern || pattern.test(name));

    if (matchedEmotes.length) {
      categories.push({
        type: "emote",
        title: "emotes",
        key: "emotes",
        emotes: matchedEmotes,
      });
    }

    if (emoji) {
      for (const group of emoji.messages.groups) {
        if (group.key === "component") {
          continue;
        }

        const matchedEmoji = emoji.emoji
          .filter((e) => e.group === group.order && testEmojiPattern(e, pattern))
          .sort((a, b) => a.order - b.order);

        if (matchedEmoji.length) {
          categories.push({
            type: "emoji",
            title: group.message,
            key: group.key,
            emoji: matchedEmoji,
          });
        }
      }
    }

    return categories;
  }, [liveEmotes, debouncedSearchQuery]);

  const [preview, setPreview] = useState<CategoryItem>(null);
  const [focusIndex, setFocusIndex] = useState(0);

  const scroller = useRef<ScrollerControl>();
  const handleNavSelect = (index: number) => {
    scroller.current.scrollToIndex(index);
  };

  const setEmojiSkinTone = (emojiSkinTone: string) => mergeUIConfig({ emojiSkinTone });

  return (
    <div
      className={clsx({
        "emote_menu": true,
        "emote_menu--search": !!debouncedSearchQuery,
      })}
    >
      <div className="emote_menu__header">
        <Search onChange={setSearchQuery} />
        <SkinTones value={uiConfig.emojiSkinTone} onSelect={setEmojiSkinTone} />
        <button className="emote_menu__close" onClick={onClose}>
          <MdClose />
        </button>
      </div>
      <div className="emote_menu__main">
        <div className="emote_menu__nav">
          <Nav categories={categories} focusIndex={focusIndex} onSelect={handleNavSelect} />
        </div>
        <div className="emote_menu__body">
          <Scroller
            className="emote_menu__scroller"
            uiConfig={uiConfig}
            categories={categories}
            onScroll={setFocusIndex}
            onSelect={onSelect}
            onHover={setPreview}
            control={scroller}
          />
          <div className="emote_menu__footer">
            <Preview meme={preview} />
          </div>
        </div>
      </div>
    </div>
  );
};

export default EmoteMenu;

const testEmojiPattern = (e: CompactEmoji, pattern?: RegExp): boolean => {
  if (!pattern) {
    return true;
  }
  for (const t of e.tags) {
    if (pattern.test(t)) {
      return true;
    }
  }
  return false;
};

interface SearchProps {
  onChange: (v: string) => void;
}

const Search: React.FC<SearchProps> = ({ onChange }) => {
  const ref = useRef<HTMLInputElement>();
  const handleChange = useStableCallback(() => onChange(ref.current.value));

  return (
    <div className="emote_menu_search">
      <input
        className="emote_menu_search__input"
        ref={ref}
        onChange={handleChange}
        placeholder="Find an emote"
      />
      <BiSearch className="emote_menu_search__icon" />
    </div>
  );
};

interface SkinTonesProps {
  value: string;
  onSelect: (tone: string) => void;
}

const SkinTones: React.FC<SkinTonesProps> = ({ value, onSelect }) => {
  const sample = "\u{1F44B}";
  const tones = ["", "\u{1F3FB}", "\u{1F3FC}", "\u{1F3FD}", "\u{1F3FE}", "\u{1F3FF}"];

  const anchor = <Emoji>{`${sample}${value}`}</Emoji>;

  const items = tones.map((tone, i) => (
    <Emoji key={i} onClick={() => onSelect(tone)}>{`${sample}${tone}`}</Emoji>
  ));

  return (
    <div className="emote_menu_skin_tones">
      <Dropdown baseClassName="emote_menu_skin_tones__dropdown" anchor={anchor} items={items} />
    </div>
  );
};

interface NavProps {
  categories: Category[];
  focusIndex: number;
  onSelect: (index: number) => void;
}

const Nav: React.FC<NavProps> = ({ categories, focusIndex, onSelect }) => {
  const icons: Record<string, React.ReactNode> = {
    "recent": <FaClock />,
    "emotes": <FaLaugh />,
    "smileys-emotion": <FaTheaterMasks />,
    "people-body": <FaWalking />,
    "animals-nature": <FaTree />,
    "food-drink": <MdFastfood />,
    "travel-places": <FaPlaneDeparture />,
    "activities": <FaBasketballBall />,
    "objects": <FaDiceD6 />,
    "symbols": <FaPeace />,
    "flags": <FaFlag />,
  };

  const items = categories.map((c, i) => (
    <li
      className={clsx({
        "emote_menu_nav__item": true,
        "emote_menu_nav__item--focus": i === focusIndex,
      })}
      key={c.key}
      onClick={() => onSelect(i)}
    >
      {icons[c.key] ?? <FaQuestionCircle />}
    </li>
  ));

  return (
    <Scrollbars className="emo">
      <ul className="emote_menu_nav">{items}</ul>
    </Scrollbars>
  );
};

interface PreviewProps {
  meme: CategoryItem;
}

const Preview: React.FC<PreviewProps> = ({ meme }) => {
  if (!meme) {
    return null;
  }
  if (meme.type === "emoji") {
    return (
      <div className="emote_menu_preview">
        <div className="emote_menu_preview__image">
          <Emoji>{meme.emoji.unicode}</Emoji>
        </div>
        <div className="emote_menu_preview__name">{meme.emoji.label}</div>
      </div>
    );
  } else {
    const { name, contributor } = meme.emote;
    return (
      <div className="emote_menu_preview">
        <div className="emote_menu_preview__image">
          <Emote name={name} />
        </div>
        <div>
          <div className="emote_menu_preview__name">{name}</div>
          {contributor?.name && (
            <div className="emote_menu_preview__contributor">
              <span className="emote_menu_preview__contributor__label">by:</span>
              <span className="emote_menu_preview__contributor__name">{contributor?.name}</span>
            </div>
          )}
        </div>
      </div>
    );
  }
};

type Category = {
  title: string;
  key: string;
} & (
  | {
      type: "emote";
      emotes: chatv1.Emote[];
    }
  | {
      type: "emoji";
      emoji: CompactEmoji[];
    }
);

type CategoryItem =
  | {
      type: "emote";
      emote: chatv1.Emote;
    }
  | {
      type: "emoji";
      emoji: CompactEmoji;
    };

interface CategoryPanelProps {
  uiConfig: chatv1.UIConfig;
  category: Category;
  width: number;
  onSelect: (v: string, send: boolean) => void;
  onHover: (v: CategoryItem) => void;
  sizeRef: React.MutableRefObject<DOMRectReadOnly>;
}

const CategoryPanel = React.memo<CategoryPanelProps>(
  ({ uiConfig, category, width, onSelect, onHover, sizeRef }) => {
    const [doubleClickTimeout, setDoubleClickTimeout] = useState(0);
    const handlePointerUp = useStableCallback((e: React.PointerEvent, v: string) => {
      if (e.button !== 0) {
        return;
      }
      if (doubleClickTimeout === 0) {
        setDoubleClickTimeout(window.setTimeout(() => onSelect(v, false), DOUBLE_CLICK_TIMEOUT));
      } else {
        clearTimeout(doubleClickTimeout);
        onSelect(v, true);
      }
    });

    const content = useMemo(() => {
      if (category.type === "emote") {
        const emoteWidths = category.emotes.map((e) => computeEmoteWidth(e) + EMOTE_MARGIN * 2);
        const rows = balanceRows(emoteWidths, width - VIEWPORT_MARGIN * 2, EMOTE_GAP);
        const emotes = rows.map((i, j) => category.emotes.slice(rows[j - 1], i));
        return emotes.map((emotes, row) => (
          <li key={row} className="emote_menu__category__list_row">
            {emotes.map((emote) => (
              <span
                key={emote.name}
                className="emote_menu__category__list_item emote_menu__category__list_item--emote"
                onPointerUp={(e) => handlePointerUp(e, emote.name)}
                onMouseEnter={() => onHover({ type: "emote", emote })}
              >
                <Emote name={emote.name} shouldAnimateForever />
              </span>
            ))}
          </li>
        ));
      } else {
        return category.emoji.map((emoji) => {
          const variant = uiConfig.emojiSkinTone
            ? emoji.skins?.find((s) => s.unicode.endsWith(uiConfig.emojiSkinTone)) ?? emoji
            : emoji;

          return (
            <li
              key={emoji.unicode}
              className="emote_menu__category__list_item emote_menu__category__list_item--emoji"
              onPointerUp={(e) => handlePointerUp(e, variant.unicode)}
              onMouseEnter={() => onHover({ type: "emoji", emoji: variant })}
            >
              <Emoji>{variant.unicode}</Emoji>
            </li>
          );
        });
      }
    }, [uiConfig, category, width]);

    const ref = useRef<HTMLDivElement>();
    sizeRef.current = useSize(ref);

    return (
      <div className="emote_menu__category" ref={ref}>
        <div className="emote_menu__category__header">{category.title}</div>
        <ul
          className={clsx(
            "emote_menu__category__list",
            `emote_menu__category__list--${category.type}`
          )}
        >
          {content}
        </ul>
      </div>
    );
  }
);

CategoryPanel.displayName = "CategoryPanel";

interface ScrollerProps {
  uiConfig: chatv1.UIConfig;
  className?: string;
  categories: Category[];
  onScroll: (index: number) => void;
  onSelect: (v: string, send: boolean) => void;
  onHover: (v: CategoryItem) => void;
  control: React.MutableRefObject<ScrollerControl>;
}

interface ScrollerControl {
  scrollToIndex(index: number): void;
}

interface ScrollbarsInternal {
  view: HTMLElement;
}

const Scroller: React.FC<ScrollerProps> = ({
  className,
  categories,
  onScroll,
  control,
  ...panelProps
}) => {
  const scrollbars = useRef<Scrollbars & ScrollbarsInternal>();
  const viewportSize = useSize(useCallback(() => scrollbars.current?.container, []));

  const sizes: React.MutableRefObject<DOMRectReadOnly>[] = [];
  const panels: React.ReactNode[] = [];
  if (viewportSize) {
    for (const category of categories) {
      const size: React.MutableRefObject<DOMRectReadOnly> = { current: null };
      sizes.push(size);
      panels.push(
        <CategoryPanel
          key={category.key}
          category={category}
          width={viewportSize.width}
          sizeRef={size}
          {...panelProps}
        />
      );
    }
  }

  useEffect(() => {
    control.current = {
      scrollToIndex: (index: number) => {
        let top = 0;
        for (let i = 0; i < index; i++) {
          top += sizes[i].current?.height ?? 0;
        }
        scrollbars.current.view.scrollTo({
          top,
          behavior: "smooth",
        });
      },
    };
  }, [control, sizes]);

  const handleScroll = useStableCallback(() => {
    const top = scrollbars.current.getScrollTop();
    let sum = 0;
    for (let i = 0; i < sizes.length; i++) {
      sum += sizes[i].current?.height ?? 0;
      if (sum > top) {
        onScroll(i);
        break;
      }
    }
  });

  return (
    <Scrollbars ref={scrollbars} onScroll={handleScroll} className={className} autoHide>
      {panels}
    </Scrollbars>
  );
};
