// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./EmoteMenu.scss";

import clsx from "clsx";
import { CompactEmoji } from "emojibase";
import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Scrollbars } from "react-custom-scrollbars-2";
import { CellMeasurer, CellMeasurerCache, Grid, List, ListRowRenderer } from "react-virtualized";

import { UIConfig } from "../../apis/strims/chat/v1/chat";
import { useChat, useRoom } from "../../contexts/Chat";
import useSize from "../../hooks/useSize";
import Emoji from "./Emoji";
import Emote from "./Emote";

interface EmoteMenuProps {
  onSelect: (v: string) => void;
}

const EmoteMenu: React.FC<EmoteMenuProps> = ({ onSelect }) => {
  const [{ uiConfig, emoji }] = useChat();
  const [{ emotes }] = useRoom();

  const categories = useMemo(() => {
    const categories: Category[] = [];
    categories.push({
      type: "emote",
      title: "emotes",
      emotes: emotes,
    });

    if (emoji) {
      for (const group of emoji.messages.groups) {
        categories.push({
          type: "emoji",
          title: group.message,
          emoji: emoji.emoji
            .filter((e) => e.group === group.order)
            .sort((a, b) => a.order - b.order),
        });
      }
    }

    return categories;
  }, [emotes]);

  const renderCategory = useCallback(
    ({ index, style }: ItemProps) => (
      <CategoryPanel
        uiConfig={uiConfig}
        category={categories[index]}
        style={style}
        onSelect={onSelect}
      />
    ),
    [categories]
  );

  return (
    <div className="emotemenu">
      <Scroller uiConfig={uiConfig} renderItem={renderCategory} itemCount={categories.length} />
    </div>
  );
};

export default EmoteMenu;

// TODO: category icon
type Category =
  | {
      type: "emote";
      title: string;
      emotes: string[];
    }
  | {
      type: "emoji";
      title: string;
      emoji: CompactEmoji[];
    };

interface CategoryPanelProps {
  uiConfig: UIConfig;
  category: Category;
  style: React.CSSProperties;
  onSelect: (v: string) => void;
}

const CategoryPanel: React.FC<CategoryPanelProps> = ({ category, style, onSelect }) => {
  const content = useMemo(() => {
    if (category.type === "emote") {
      return category.emotes.map((name) => (
        <li
          key={name}
          className="emotemenu__category__list_item emotemenu__category__list_item--emote"
          onClick={() => onSelect(name)}
        >
          <Emote name={name} shouldAnimateForever />
        </li>
      ));
    }
    return category.emoji.map(({ unicode }) => (
      <li
        key={unicode}
        className="emotemenu__category__list_item emotemenu__category__list_item--emoji"
        onClick={() => onSelect(unicode)}
      >
        <Emoji>{unicode}</Emoji>
      </li>
    ));
  }, [category]);

  return (
    <div style={style}>
      <div className="emotemenu__category__header">{category.title}</div>
      <ul
        className={clsx("emotemenu__category__list", `emotemenu__category__list--${category.type}`)}
      >
        {content}
      </ul>
    </div>
  );
};

interface ItemProps {
  style: React.CSSProperties;
  index: number;
}

interface ScrollerProps {
  uiConfig: UIConfig;
  renderItem: (ItemProps) => React.ReactNode;
  itemCount: number;
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

const Scroller: React.FC<ScrollerProps> = ({ uiConfig, itemCount, renderItem }) => {
  const list = useRef<List & ListInternal>();
  const scrollbars = useRef<Scrollbars & ScrollbarsInternal>();
  const sizeCache = useMemo(() => new CellMeasurerCache({ fixedWidth: true }), []);

  useEffect(() => {
    list.current.Grid._scrollingContainer = scrollbars.current.view;
  }, []);

  const size = useSize(scrollbars.current?.container);
  const width = size?.width ?? 0;
  const height = size?.height ?? 0;

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
        {renderItem({ index, style })}
      </CellMeasurer>
    ),
    [renderItem, width]
  );

  const [scrolling, setScrolling] = useState(true);
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
    <>
      <Scrollbars
        ref={scrollbars}
        onScroll={handleScroll}
        onScrollStart={handleScrollStart}
        onScrollStop={handleScrollStop}
        renderThumbVertical={renderScrollThumb}
        onMouseEnter={handleScrollMouseEnter}
        onMouseLeave={handleScrollMouseLeave}
      >
        <List
          ref={list}
          height={height}
          width={width}
          style={{ overflowX: "visible", overflowY: "visible" }}
          deferredMeasurementCache={sizeCache}
          rowHeight={sizeCache.rowHeight}
          rowCount={itemCount}
          rowRenderer={renderRow}
        />
      </Scrollbars>
    </>
  );
};
