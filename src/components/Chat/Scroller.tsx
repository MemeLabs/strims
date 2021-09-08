import clsx from "clsx";
import React, { CSSProperties, ReactNode, useCallback, useEffect, useRef, useState } from "react";
import { Scrollbars } from "react-custom-scrollbars-2";
import { useDebounce } from "react-use";
import {
  AutoSizer,
  CellMeasurer,
  CellMeasurerCache,
  Dimensions,
  Grid,
  List,
  ListRowRenderer,
  OnScrollParams,
} from "react-virtualized";

import { UIConfig } from "../../apis/strims/chat/v1/chat";

export interface MessageProps {
  style: CSSProperties;
  index: number;
}

interface ScrollerProps {
  uiConfig: UIConfig;
  renderMessage: (MessageProps) => ReactNode;
  messageCount: number;
  messageSizeCache: CellMeasurerCache;
  autoScrollThreshold?: number;
  resizeDebounceTimeout?: number;
}

const Scroller: React.FC<ScrollerProps> = (props) => {
  return (
    <AutoSizer>
      {(dimensions: Dimensions) => <ScrollerContent {...dimensions} {...props} />}
    </AutoSizer>
  );
};

interface ListInternal {
  Grid: Grid & {
    _scrollingContainer: HTMLElement;
    _onScroll: (e: UIEvent) => void;
  };
}

interface ScrollbarsInternal {
  view: HTMLElement;
}

const ScrollerContent: React.FC<ScrollerProps & Dimensions> = ({
  uiConfig,
  height,
  width,
  messageCount,
  renderMessage,
  messageSizeCache,
  autoScrollThreshold = 20,
  resizeDebounceTimeout = 100,
}) => {
  const list = useRef<List & ListInternal>();
  const scrollbars = useRef<Scrollbars & ScrollbarsInternal>();
  const [autoScroll, setAutoScroll] = useState(true);
  const [scrolling, setScrolling] = useState(true);
  const [resizing, setResizing] = useState(true);

  const autoScrollEvent = useRef(false);
  const applyAutoScroll = () => {
    if (autoScroll) {
      autoScrollEvent.current = true;
      list.current?.scrollToRow(list.current.props.rowCount);
    }
  };

  const recomputeRowHeights = () => {
    messageSizeCache.clearAll();
    list.current?.recomputeRowHeights();
    applyAutoScroll();
  };

  useDebounce(recomputeRowHeights, 500, [width]);
  useEffect(recomputeRowHeights, [uiConfig]);
  useEffect(applyAutoScroll, [autoScroll, messageCount, height]);

  useEffect(() => {
    setResizing(true);
    const id = setTimeout(() => setResizing(false), resizeDebounceTimeout);
    return () => clearTimeout(id);
  }, [height, width]);

  useEffect(() => {
    list.current.Grid._scrollingContainer = scrollbars.current.view;
  }, []);

  const handleScroll = useCallback((e) => {
    // autoscroll acts on react-virtualized directly so there is no need to
    // forward the scroll event observed by react-custom-scrollbars. the list
    // only needs to be updated in response to user scrolling events.

    if (!autoScrollEvent.current) {
      list.current?.Grid?._onScroll(e);
    }
    autoScrollEvent.current = false;
  }, []);
  const handleScrollStart = useCallback(() => setScrolling(true), []);
  const handleScrollStop = useCallback(() => setScrolling(false), []);

  const handleListScroll = useCallback(
    ({ scrollHeight, scrollTop, clientHeight }: OnScrollParams) => {
      const thresholdExceeded = scrollHeight - scrollTop - clientHeight < autoScrollThreshold;
      const isScrolling = list.current?.Grid?.state.isScrolling;
      const enabled = !isScrolling ? autoScroll : thresholdExceeded;

      if (resizing && enabled) {
        list.current?.scrollToRow(list.current.props.rowCount);
      }
      setAutoScroll(enabled);
    },
    [autoScroll, resizing]
  );

  const renderRow: ListRowRenderer = useCallback(
    ({ index, key, style, parent }) => (
      <CellMeasurer
        cache={messageSizeCache}
        columnIndex={0}
        key={key}
        parent={parent}
        rowIndex={index}
        width={width}
      >
        {renderMessage({ index, style })}
      </CellMeasurer>
    ),
    [renderMessage, messageSizeCache, width]
  );

  const renderScrollThumb = useCallback(
    (props) => (
      <div
        {...props}
        className={clsx({
          "chat__scroller__scrollbar_handle": true,
          "chat__scroller__scrollbar_handle--scrolling": scrolling && !autoScroll,
        })}
      />
    ),
    [scrolling, autoScroll]
  );

  return (
    <Scrollbars
      ref={scrollbars}
      onScroll={handleScroll}
      onScrollStart={handleScrollStart}
      onScrollStop={handleScrollStop}
      style={{ height, width }}
      renderThumbVertical={renderScrollThumb}
    >
      <List
        ref={list}
        height={height}
        width={width}
        style={{ overflowX: "visible", overflowY: "visible" }}
        deferredMeasurementCache={messageSizeCache}
        rowHeight={messageSizeCache.rowHeight}
        rowCount={messageCount}
        rowRenderer={renderRow}
        onScroll={handleListScroll}
      />
    </Scrollbars>
  );
};

export default Scroller;
