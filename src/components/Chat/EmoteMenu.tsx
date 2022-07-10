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
import { MdFastfood } from "react-icons/md";
import { useDebounce } from "react-use";
import { CellMeasurer, CellMeasurerCache, Grid, List, ListRowRenderer } from "react-virtualized";
import { RenderedRows } from "react-virtualized/dist/es/List";

import * as chatv1 from "../../apis/strims/chat/v1/chat";
import { useChat, useRoom } from "../../contexts/Chat";
import useSize from "../../hooks/useSize";
import { useStableCallback } from "../../hooks/useStableCallback";
import Dropdown from "../Dropdown";
import Emoji from "./Emoji";
import Emote from "./Emote";

interface EmoteMenuProps {
  onSelect: (v: string) => void;
}

const EmoteMenu: React.FC<EmoteMenuProps> = ({ onSelect }) => {
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

  const [preview, setPreview] = useState<Meme>(null);
  const [focusIndex, setFocusIndex] = useState(0);

  const scroller = useRef<ScrollerRef>();
  const handleNavSelect = (index: number) => scroller.current.scrollToIndex(index);

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
            ref={scroller}
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

  return <ul className="emote_menu_nav">{items}</ul>;
};

interface PreviewProps {
  meme: Meme;
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
        <div>{meme.emoji.label}</div>
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
          <div>{name}</div>
          <div>{contributor?.name}</div>
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

type Meme =
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
  style: React.CSSProperties;
  onSelect: (v: string) => void;
  onHover: (v: Meme) => void;
}

const CategoryPanel: React.FC<CategoryPanelProps> = ({
  uiConfig,
  category,
  style,
  onSelect,
  onHover,
}) => {
  const content = useMemo(() => {
    if (category.type === "emote") {
      return category.emotes.map((emote) => (
        <li
          key={emote.name}
          className="emote_menu__category__list_item emote_menu__category__list_item--emote"
          onClick={() => onSelect(emote.name)}
          onMouseEnter={() => onHover({ type: "emote", emote })}
        >
          <Emote name={emote.name} shouldAnimateForever />
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
            onClick={() => onSelect(variant.unicode)}
            onMouseEnter={() => onHover({ type: "emoji", emoji: variant })}
          >
            <Emoji>{variant.unicode}</Emoji>
          </li>
        );
      });
    }
  }, [uiConfig, category]);

  return (
    <div className="emote_menu__category" style={style}>
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
};

interface ScrollerProps {
  uiConfig: chatv1.UIConfig;
  className?: string;
  categories: Category[];
  onScroll: (index: number) => void;
  onSelect: (v: string) => void;
  onHover: (v: Meme) => void;
}

interface ScrollerRef {
  scrollToIndex(index: number): void;
}

interface ListInternal {
  Grid: Grid & {
    _scrollingContainer: HTMLElement;
    _onScroll: (e: React.UIEvent) => void;
  };
}

interface ScrollbarsInternal {
  view: HTMLElement;
}

const Scroller = React.forwardRef<ScrollerRef, ScrollerProps>(
  ({ className, uiConfig, categories, onScroll, onSelect, onHover }, ref) => {
    const list = useRef<List & ListInternal>();
    const scrollbars = useRef<Scrollbars & ScrollbarsInternal>();
    const sizeCache = useMemo(() => new CellMeasurerCache({ fixedWidth: true }), []);

    useEffect(() => {
      const current = {
        scrollToIndex: (index: number) => {
          let top = 0;
          for (let i = 0; i < index; i++) {
            top += sizeCache.getHeight(i, 0);
          }

          scrollbars.current.view.scrollTo({
            top,
            behavior: "smooth",
          });
        },
      };

      if (ref instanceof Function) {
        ref(current);
      } else if (ref) {
        ref.current = current;
      }
    }, [ref]);

    useEffect(() => {
      list.current.Grid._scrollingContainer = scrollbars.current.view;
    }, []);

    const size = useSize(useCallback(() => scrollbars.current?.container, []));
    const width = size?.width ?? 0;
    const height = size?.height ?? 0;

    useEffect(() => {
      sizeCache.clearAll();
      list.current.recomputeRowHeights();
    }, [width, height, categories]);

    const handleScroll: React.UIEventHandler = useCallback((e) => {
      list.current?.Grid?._onScroll(e);
    }, []);

    const renderRow: ListRowRenderer = useCallback(
      ({ index, key, style, parent }) => (
        <CellMeasurer
          cache={sizeCache}
          columnIndex={0}
          key={key}
          parent={parent}
          rowIndex={index}
          width={width}
        >
          <CategoryPanel
            uiConfig={uiConfig}
            category={categories[index]}
            style={style}
            onSelect={onSelect}
            onHover={onHover}
          />
        </CellMeasurer>
      ),
      [uiConfig, categories, width]
    );

    const handleRowsRendered = useStableCallback(({ startIndex }: RenderedRows) =>
      onScroll(startIndex)
    );

    const [scrolling, setScrolling] = useState(false);
    const [hovering, setHovering] = useState(false);

    const handleScrollStart = useCallback(() => setScrolling(true), []);
    const handleScrollStop = useCallback(() => setScrolling(false), []);
    const handleScrollMouseEnter = useCallback(() => setHovering(true), []);
    const handleScrollMouseLeave = useCallback(() => setHovering(false), []);

    const renderScrollThumb = useCallback(
      (props) => (
        <div
          {...props}
          className={clsx({
            "chat__scroller__scrollbar_handle": true,
            "chat__scroller__scrollbar_handle--scrolling": scrolling || hovering,
          })}
        />
      ),
      [scrolling, hovering]
    );

    return (
      <Scrollbars
        ref={scrollbars}
        onScroll={handleScroll}
        onScrollStart={handleScrollStart}
        onScrollStop={handleScrollStop}
        renderThumbVertical={renderScrollThumb}
        onMouseEnter={handleScrollMouseEnter}
        onMouseLeave={handleScrollMouseLeave}
        className={className}
      >
        <List
          ref={list}
          height={height}
          width={width}
          style={{ overflowX: "visible", overflowY: "visible" }}
          deferredMeasurementCache={sizeCache}
          rowHeight={sizeCache.rowHeight}
          rowCount={categories.length}
          overscanRowCount={5}
          rowRenderer={renderRow}
          onRowsRendered={handleRowsRendered}
        />
      </Scrollbars>
    );
  }
);
