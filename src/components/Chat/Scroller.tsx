// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Scroller.scss";

import clsx from "clsx";
import React, { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import Scrollbars from "react-custom-scrollbars-2";
import { Trans } from "react-i18next";
import { FiArrowDownCircle } from "react-icons/fi";
import { usePrevious } from "react-use";

import { useRoom } from "../../contexts/Chat";
import useRefs from "../../hooks/useRefs";
import useSize from "../../hooks/useSize";
import { useStableCallback } from "../../hooks/useStableCallback";
import MessageSizeCache from "../../lib/MessageSizeCache";

export interface MessageProps {
  index: number;
  style: React.CSSProperties;
  ref: React.Ref<HTMLDivElement>;
}

export type MessageRenderFunc = (props: MessageProps) => React.ReactElement;

interface ScrollerProps extends React.ComponentProps<"div"> {
  renderMessage: MessageRenderFunc;
  messageCount: number;
  messageSizeCache: MessageSizeCache;
  overscan?: number;
  autoScrollThreshold?: number;
}

interface ScrollerViewProps extends ScrollerProps {
  viewportHeight: number;
  viewportWidth: number;
  onAutoscrollChange: (v: boolean) => void;
  autoScroll: boolean;
}

const ScrollerView = React.forwardRef<HTMLDivElement, ScrollerViewProps>(
  (
    {
      renderMessage,
      messageCount,
      messageSizeCache,
      onAutoscrollChange,
      autoScroll,
      viewportHeight = 0,
      viewportWidth = 0,
      overscan = 10,
      autoScrollThreshold = 20,
      ...props
    },
    fwRef
  ) => {
    const [scrollTop, setScrollTop] = useState(0);
    const pruned = usePrevious(messageCount) > messageCount;
    const prevViewportWidth = usePrevious(viewportWidth);
    const reset = prevViewportWidth !== undefined && prevViewportWidth !== viewportWidth;

    if (reset) {
      messageSizeCache.reset();
    }
    messageSizeCache.grow(messageCount);

    messageSizeCache.onchange = () => {
      if (autoScroll) {
        setScrollTop(messageSizeCache.getOffset(messageCount) - viewportHeight);
      }
    };
    useEffect(() => () => (messageSizeCache.onchange = null), []);

    const indexRange = useMemo(() => {
      const start = Math.max(0, messageSizeCache.findIndex(scrollTop) - overscan);
      const end = Math.min(
        messageCount,
        messageSizeCache.findIndex(scrollTop + viewportHeight) + overscan
      );
      return { start, end };
    }, [reset, pruned, scrollTop, viewportHeight]);

    const children = useMemo(() => {
      const children: React.ReactElement[] = [];
      for (let i = indexRange.start; i < indexRange.end; i++) {
        children.push(
          <MessageMeasurer
            key={i}
            index={i}
            renderMessage={renderMessage}
            messageSizeCache={messageSizeCache}
            unstable={i === indexRange.start || i === indexRange.end - 1}
            offset={messageSizeCache.getOffset(i)}
            settled={messageSizeCache.isSettled(i)}
          />
        );
      }
      return children;
    }, [renderMessage, indexRange, messageSizeCache.getOffset(indexRange.end)]);

    const ref = useRef<HTMLDivElement>();
    const ignoreScroll = useRef(false);
    const height = messageSizeCache.getOffset(messageCount);
    const [, { toggleMessageGC }] = useRoom();

    useLayoutEffect(() => {
      if (autoScroll) {
        ignoreScroll.current = true;

        ref.current?.scrollTo({ top: height });
        setScrollTop(height - viewportHeight);
      }
    }, [height, autoScroll, viewportHeight]);

    const handleScroll = useStableCallback(() => {
      // ignore scroll events triggered by autoscroll
      if (ignoreScroll.current) {
        ignoreScroll.current = false;
        return;
      }

      // ignore scroll events until the viewport resize event resolves to avoid
      // ending autoscroll unintentionally
      if (autoScroll && ref.current.parentElement.offsetHeight !== Math.round(viewportHeight)) {
        return;
      }

      const { scrollTop } = ref.current;
      const thresholdExceeded = height - scrollTop - viewportHeight < autoScrollThreshold;
      if (thresholdExceeded !== autoScroll) {
        toggleMessageGC(thresholdExceeded);
        onAutoscrollChange(thresholdExceeded);
      }
      setScrollTop(scrollTop);
    });

    const style = useMemo<React.CSSProperties>(() => {
      return {
        width: "100%",
        height: `${height}px`,
      };
    }, [height]);

    return (
      <div onScroll={handleScroll} {...props} ref={useRefs(ref, fwRef)}>
        <div style={style}>{children}</div>
      </div>
    );
  }
);

ScrollerView.displayName = "Scroller.ScrollerView";

const Scroller: React.FC<ScrollerProps> = (props) => {
  const ref = useRef<Scrollbars>(null);
  const size = useSize(useCallback(() => ref.current?.container, []));
  const [, { resetSelectedPeers, toggleMessageGC }] = useRoom();
  const [autoScroll, setAutoScroll] = useState(true);
  const [scrolling, setScrolling] = useState(true);
  const [hovering, setHovering] = useState(false);

  const handleScrollStart = useCallback(() => setScrolling(true), []);
  const handleScrollStop = useCallback(() => setScrolling(false), []);
  const handleScrollMouseEnter = useCallback(() => setHovering(true), []);
  const handleScrollMouseLeave = useCallback(() => setHovering(false), []);
  const handleClick = useStableCallback(() => resetSelectedPeers());
  const handleResumeClick = useStableCallback(() => {
    toggleMessageGC(true);
    setAutoScroll(true);
  });

  const renderScrollThumb = (props) => (
    <div
      {...props}
      className={clsx({
        "chat__scroller__scrollbar_handle": true,
        "chat__scroller__scrollbar_handle--scrolling": (scrolling && !autoScroll) || hovering,
      })}
    />
  );

  const renderView = (renderProps) => (
    <ScrollerView
      {...props}
      {...renderProps}
      viewportHeight={size?.height}
      viewportWidth={size?.width}
      autoScroll={autoScroll}
      onAutoscrollChange={setAutoScroll}
    />
  );

  return (
    <>
      <Scrollbars
        ref={ref}
        style={{ overflowX: "hidden" }}
        renderView={renderView}
        renderThumbVertical={renderScrollThumb}
        onScrollStart={handleScrollStart}
        onScrollStop={handleScrollStop}
        onMouseEnter={handleScrollMouseEnter}
        onMouseLeave={handleScrollMouseLeave}
        onClick={handleClick}
      />
      {!autoScroll && (
        <div className="chat__scroller__resume_autoscroll" onClick={handleResumeClick}>
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

interface MessageMeasurerProps {
  index: number;
  renderMessage: MessageRenderFunc;
  messageSizeCache: MessageSizeCache;
  unstable: boolean;
  offset: number;
  settled: boolean;
}

const MessageMeasurer = React.memo<MessageMeasurerProps>(
  ({ index, renderMessage, messageSizeCache, unstable, offset, settled }) => {
    const ref = useRef<HTMLDivElement>();

    useLayoutEffect(() => {
      if (!settled) {
        const rect = ref.current.getBoundingClientRect();
        const { marginTop, marginBottom } = window.getComputedStyle(ref.current);
        messageSizeCache.set(
          index,
          !unstable,
          rect.height,
          parseFloat(marginTop),
          parseFloat(marginBottom)
        );
      }
    }, [unstable, settled]);

    const style = useMemo<React.CSSProperties>(() => {
      return {
        position: "absolute",
        top: `${offset}px`,
      };
    }, [offset]);

    return renderMessage({ index, ref, style });
  },
  (prev, next) =>
    next.settled &&
    prev.index === next.index &&
    prev.renderMessage === next.renderMessage &&
    prev.messageSizeCache === next.messageSizeCache &&
    prev.unstable === next.unstable &&
    prev.offset === next.offset &&
    prev.settled === next.settled
);

MessageMeasurer.displayName = "Scroller.MessageMeasurer";
