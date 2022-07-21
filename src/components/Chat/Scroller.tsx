// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Scroller.scss";

import clsx from "clsx";
import React, { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import Scrollbars from "react-custom-scrollbars-2";
import { Trans } from "react-i18next";
import { FiArrowDownCircle } from "react-icons/fi";
import { useFirstMountState, usePrevious } from "react-use";

import { UIConfig } from "../../apis/strims/chat/v1/chat";
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
  uiConfig: UIConfig;
  renderMessage: MessageRenderFunc;
  messageCount: number;
  messageSizeCache: MessageSizeCache;
  overscan?: number;
  autoScrollThreshold?: number;
}

interface ScrollerViewProps extends ScrollerProps {
  height: number;
  width: number;
  onAutoscrollChange: (v: boolean) => void;
  autoScroll: boolean;
}

const ScrollerView = React.forwardRef<HTMLDivElement, ScrollerViewProps>(
  (
    {
      uiConfig,
      renderMessage,
      messageCount,
      messageSizeCache,
      onAutoscrollChange,
      autoScroll,
      height = 0,
      width = 0,
      overscan = 10,
      autoScrollThreshold = 20,
      ...props
    },
    fwRef
  ) => {
    const [, { toggleMessageGC }] = useRoom();
    const [scrollTop, setScrollTop] = useState(0);

    messageSizeCache.grow(messageCount);

    const offset = messageSizeCache.getOffset(messageCount);
    const pruned = usePrevious(messageCount) > messageCount;

    const indexRange = useMemo(() => {
      const start = Math.max(0, messageSizeCache.findIndex(scrollTop) - overscan);
      const end = Math.min(messageCount, messageSizeCache.findIndex(scrollTop + height) + overscan);
      return { start, end };
    }, [pruned, scrollTop, height]);

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
    const firstMount = useFirstMountState();
    const ignoreScroll = useRef(false);
    const [style, setStyle] = useState<React.CSSProperties>({ width: "100%" });

    useLayoutEffect(() => {
      if (!firstMount) {
        messageSizeCache.reset();

        if (!autoScroll) {
          const offset = messageSizeCache.getOffset(indexRange.start + overscan);
          ref.current.scrollTo({ top: offset });
          setScrollTop(offset);
        }
      }
    }, [width, uiConfig]);

    const syncOffset = useStableCallback(() => {
      setStyle({
        width: "100%",
        height: `${offset}px`,
      });

      if (autoScroll && ref.current) {
        ignoreScroll.current = true;
        ref.current.scrollTo({ top: offset });
        setScrollTop(offset - height);
      }
    });

    useLayoutEffect(syncOffset, [offset, autoScroll, height]);

    useEffect(() => {
      messageSizeCache.onchange = syncOffset;
      return () => (messageSizeCache.onchange = null);
    }, []);

    const handleScroll = useStableCallback((e) => {
      if (ignoreScroll.current) {
        ignoreScroll.current = false;
        return;
      }

      const { scrollTop } = ref.current;
      const thresholdExceeded = offset - scrollTop - height < autoScrollThreshold;
      if (thresholdExceeded !== autoScroll) {
        toggleMessageGC(thresholdExceeded);
        onAutoscrollChange(thresholdExceeded);
      }
      setScrollTop(scrollTop);
    });

    return (
      <div className="scroller2" onScroll={handleScroll} {...props} ref={useRefs(ref, fwRef)}>
        <div style={style}>{children}</div>
      </div>
    );
  }
);

ScrollerView.displayName = "Scroller.ScrollerView";

const Scroller: React.FC<ScrollerProps> = (props) => {
  const ref = useRef<Scrollbars>(null);
  const size = useSize(useCallback(() => ref.current?.container, []));
  const [autoScroll, setAutoScroll] = useState(true);
  const [, { resetSelectedPeers, toggleMessageGC }] = useRoom();

  const [hovering, setHovering] = useState(false);

  const handleScrollMouseEnter = useCallback(() => setHovering(true), []);
  const handleScrollMouseLeave = useCallback(() => setHovering(false), []);
  const handleClick = useStableCallback(() => resetSelectedPeers());

  const renderScrollThumb = useCallback(
    (props) => (
      <div
        {...props}
        className={clsx({
          "chat__scroller__scrollbar_handle": true,
          "chat__scroller__scrollbar_handle--scrolling": hovering,
        })}
      />
    ),
    [hovering]
  );

  const handleResumeClick = useStableCallback(() => {
    toggleMessageGC(true);
    setAutoScroll(true);
  });

  const renderView = (renderProps) => (
    <ScrollerView
      {...props}
      {...renderProps}
      height={size?.height}
      width={size?.width}
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
        width: "100%",
        top: `${offset}px`,
      };
    }, [offset]);

    return renderMessage({ index, ref, style });
  }
);

MessageMeasurer.displayName = "Scroller.MessageMeasurer";
