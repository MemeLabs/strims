import clsx from "clsx";
import { debounce } from "lodash";
import React, { CSSProperties, ReactNode, useCallback, useEffect, useRef, useState } from "react";
import { Scrollbars } from "react-custom-scrollbars-2";
import { Trans } from "react-i18next";
import { FiArrowDownCircle } from "react-icons/fi";
import { useDebounce, useUpdateEffect } from "react-use";
import {
  CellMeasurer,
  CellMeasurerCache,
  Grid,
  List,
  ListRowRenderer,
  OnScrollParams,
} from "react-virtualized";
import { RenderedRows } from "react-virtualized/dist/es/List";

import { UIConfig } from "../../apis/strims/chat/v1/chat";
import useSize from "../../hooks/useSize";
import { retrySync } from "../../lib/retry";

const AUTOSCROLL_THRESHOLD = 20;
const RESIZE_DEBOUNCE_TIMEOUT = 200;

export interface MessageProps {
  style: CSSProperties;
  index: number;
}

interface ScrollerProps {
  uiConfig: UIConfig;
  renderMessage: (MessageProps) => ReactNode;
  messageCount: number;
  messageSizeCache: CellMeasurerCache;
  onAutoScrollChange?: (state: boolean) => void;
}

interface ListInternal {
  Grid: Grid & {
    _scrollingContainer: HTMLElement;
    _onScroll: (e: UIEvent) => void;
  };
}

interface ScrollbarsInternal {
  view: HTMLElement;
}

const Scroller: React.FC<ScrollerProps> = ({
  uiConfig,
  messageCount,
  renderMessage,
  messageSizeCache,
  onAutoScrollChange,
}) => {
  const list = useRef<List & ListInternal>();
  const scrollbars = useRef<Scrollbars & ScrollbarsInternal>();

  useEffect(() => {
    list.current.Grid._scrollingContainer = scrollbars.current.view;
  }, []);

  const size = useSize(scrollbars.current?.container);
  const width = size?.width ?? 0;
  const height = size?.height ?? 0;

  const [autoScroll, setAutoScroll] = useState(true);
  const state = useRef({
    resizing: false,
    scrollbarHeight: 0,
    scrollbarWidth: 0,
    index: 0,
    autoScrollEvent: false,
    recomputingRowHeights: false,
  }).current;

  const forceAutoScroll = () => {
    state.autoScrollEvent = true;
    list.current?.scrollToRow(list.current.props.rowCount);
  };

  const applyAutoScroll = () => {
    if (autoScroll) {
      forceAutoScroll();
    }
  };

  const recomputeRowHeights = () => {
    let getRow = () => list.current.props.rowCount - 1;
    if (!autoScroll) {
      const { index } = state;
      getRow = () => index;
    }

    state.recomputingRowHeights = true;
    messageSizeCache.clearAll();
    list.current?.recomputeRowHeights();

    // row heights are recomputed asynchronously and the api gives no indication
    // of when the work is complete. attempting to scroll to a row whose
    // position has not been computed causes the scroll top to revert to zero.
    // to avoid this we poll the size cache until it's safe to fix up the scroll
    // position.
    const scrollToRow = () => {
      const ready = messageSizeCache.has(getRow(), 0);
      if (ready) {
        state.autoScrollEvent = true;
        list.current?.scrollToRow(getRow());
        state.recomputingRowHeights = false;
        state.resizing = false;
      }
      return ready;
    };
    retrySync(scrollToRow, 10, 10);
  };

  useDebounce(recomputeRowHeights, RESIZE_DEBOUNCE_TIMEOUT, [width]);
  useUpdateEffect(recomputeRowHeights, [uiConfig]);
  useEffect(applyAutoScroll, [messageCount]);

  const handleScroll = useCallback((e) => {
    // autoscroll acts on react-virtualized directly so there is no need to
    // forward the scroll event observed by react-custom-scrollbars. the list
    // only needs to be updated in response to user scroll events.
    if (!state.autoScrollEvent) {
      list.current?.Grid?._onScroll(e);
    }
    state.autoScrollEvent = false;
  }, []);

  const clearResizing = useCallback(
    debounce(() => (state.resizing = false), RESIZE_DEBOUNCE_TIMEOUT),
    []
  );

  // during list scroll events determine whether the user has scrolled to within
  // the autoscroll threshold and update the autoscroll state accordingly.
  // during automatically triggered scroll events fix up the scroll position.
  const handleListScroll = useCallback(
    ({ scrollHeight, scrollTop, clientHeight }: OnScrollParams) => {
      // when the container is resized scroll events fire but the order the
      // scroll and resize events are triggered isn't predictable. to work
      // around this we manually track the scrollbar container size. if we
      // detect a resize events we should avoid toggling autoscroll. if
      // autoscroll is enabled we should fix up the scroll position.
      const scrollbarHeight = scrollbars.current?.container?.scrollHeight;
      const scrollbarWidth = scrollbars.current?.container?.scrollWidth;
      if (
        scrollbarHeight !== (state.scrollbarHeight ?? scrollbarHeight) ||
        scrollbarWidth !== (state.scrollbarWidth ?? scrollbarWidth)
      ) {
        state.resizing = true;
        clearResizing();
      }
      state.scrollbarHeight = scrollbarHeight;
      state.scrollbarWidth = scrollbarWidth;

      // while the row heights are being recomputed changing the scroll position
      // or behavior will cause conflicts because the scroll offset can't be
      // determined.
      if (state.recomputingRowHeights) {
        return;
      }

      const thresholdExceeded = scrollHeight - scrollTop - clientHeight < AUTOSCROLL_THRESHOLD;
      const scrolling = list.current?.Grid?.state.isScrolling;
      const enabled = state.resizing || !scrolling ? autoScroll : thresholdExceeded;

      if (state.resizing && enabled) {
        forceAutoScroll();
      }

      if (autoScroll !== enabled) {
        setAutoScroll(enabled);
        onAutoScrollChange?.(enabled);
      }
    },
    [autoScroll]
  );

  const handleListRowsRendered = useCallback(({ stopIndex }: RenderedRows) => {
    state.index = stopIndex;
  }, []);

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
          "chat__scroller__scrollbar_handle--scrolling": (scrolling && !autoScroll) || hovering,
        })}
      />
    ),
    [scrolling, autoScroll, hovering]
  );

  const resumeAutoScroll = useCallback(() => {
    setAutoScroll(true);
    onAutoScrollChange?.(true);
    forceAutoScroll();
  }, []);

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
          deferredMeasurementCache={messageSizeCache}
          rowHeight={messageSizeCache.rowHeight}
          rowCount={messageCount}
          rowRenderer={renderRow}
          onScroll={handleListScroll}
          onRowsRendered={handleListRowsRendered}
        />
      </Scrollbars>
      {!autoScroll && (
        <div className="chat__scroller__resume_autoscroll" onClick={resumeAutoScroll}>
          <span>
            <Trans>chat.More messages below</Trans>
          </span>
          <FiArrowDownCircle />
        </div>
      )}
    </>
  );
};

export default Scroller;
