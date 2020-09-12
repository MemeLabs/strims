import clsx from "clsx";
import React from "react";
import { FunctionComponent } from "react";
import { Scrollbars } from "react-custom-scrollbars";
import {
  AutoSizer,
  CellMeasurer,
  CellMeasurerCache,
  Dimensions,
  Grid,
  List,
  OnScrollParams,
} from "react-virtualized";

import { ChatClientEvent } from "../../lib/pb";
import Message from "./Message";

interface ScrollerProps {
  messages: ChatClientEvent.IMessage[];
}

const Scroller: FunctionComponent<ScrollerProps> = ({ messages }) => {
  return (
    <AutoSizer>
      {(dimensions: Dimensions) => <ScrollerContent {...dimensions} messages={messages} />}
    </AutoSizer>
  );
};

interface UnsafeList extends List {
  Grid: Grid & {
    _scrollingContainer: any;
    _onScroll: any;
  };
}

interface UnsafeScrollbars extends Scrollbars {
  view: any;
}

const ScrollerContent: FunctionComponent<ScrollerProps & Dimensions> = ({
  height,
  width,
  messages,
}) => {
  const list = React.useRef<UnsafeList>();
  const scrollbars = React.useRef<UnsafeScrollbars>();
  const cache = React.useMemo(() => new CellMeasurerCache({ fixedWidth: true }), []);
  const [autoScroll, setAutoScroll] = React.useState(true);
  const [scrolling, setScrolling] = React.useState(true);

  React.useEffect(() => {
    cache.clearAll();
    list.current?.recomputeRowHeights();
  }, [list, height, width]);

  React.useEffect(() => {
    if (autoScroll) {
      list.current?.scrollToRow(messages.length - 1);
    }
  }, [list, messages, height, width]);

  React.useEffect(() => {
    if (list.current && scrollbars.current) {
      list.current.Grid._scrollingContainer = scrollbars.current.view;
    }
  }, [list, scrollbars]);

  const handleScroll = React.useCallback((e) => list.current?.Grid?._onScroll(e), [list]);
  const handleScrollStart = React.useCallback(() => setScrolling(true), []);
  const handleScrollStop = React.useCallback(() => setScrolling(false), []);

  const handleListScroll = React.useCallback(
    (e: OnScrollParams) => setAutoScroll(e.scrollHeight - e.scrollTop - e.clientHeight < 20),
    []
  );

  const renderRow = React.useCallback(
    ({ index, key, style, parent }) => (
      <CellMeasurer
        cache={cache}
        columnIndex={0}
        key={key}
        parent={parent}
        rowIndex={index}
        width={width}
      >
        <Message message={messages[index]} style={style} />
      </CellMeasurer>
    ),
    [messages, width]
  );

  const renderScrollThumb = React.useCallback(
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
        deferredMeasurementCache={cache}
        rowHeight={cache.rowHeight}
        rowCount={messages.length}
        rowRenderer={renderRow}
        onScroll={handleListScroll}
      />
    </Scrollbars>
  );
};

export default Scroller;
