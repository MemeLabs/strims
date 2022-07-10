// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "swiper/css";

import "./RoomCarousel.scss";

import { useDrag } from "@use-gesture/react";
import clsx from "clsx";
import { Base64 } from "js-base64";
import { isEqual } from "lodash";
import React, { useCallback, useRef } from "react";
import { FreeMode, Mousewheel } from "swiper";
import { Swiper, SwiperSlide } from "swiper/react";

import { ThreadProviderProps, ThreadState, useChat } from "../../contexts/Chat";
import useSize from "../../hooks/useSize";
import { useStableCallback } from "../../hooks/useStableCallback";
import { DEVICE_TYPE, DeviceType } from "../../lib/userAgent";
import Badge from "../Badge";
import { MenuItem, useContextMenu } from "../ContextMenu";

interface RoomCarouselGemProps extends ThreadProviderProps {
  color: string;
  label: string;
  unreadCount: number;
  onChange: (topic: ThreadProviderProps) => void;
  selected: boolean;
}

const RoomCarouselGem: React.FC<RoomCarouselGemProps> = ({
  color,
  label,
  unreadCount,
  type,
  topicKey,
  onChange,
  selected,
}) => {
  const topic = { type, topicKey };
  const [, chatActions] = useChat();

  const ref = useRef<HTMLDivElement>();
  useDrag(
    ({ movement: [, my], swipe: [, sy], dragging, first }) => {
      ref.current.style.setProperty("--drag-offset", `${my}px`);

      if (!dragging) {
        ref.current.classList.remove("room_carousel__item--dragging");
        ref.current.style.removeProperty("--drag-offset");
      } else if (first) {
        ref.current.classList.add("room_carousel__item--dragging");
      }

      if (sy === -1) {
        chatActions.closeTopic(topic);
      }
    },
    {
      enabled: true,
      axis: "y",
      target: ref,
      swipe: {
        distance: [20, 20],
      },
      pointer: {
        touch: true,
        capture: true,
      },
    }
  );

  const handleClick = useStableCallback(() => onChange(topic));

  const { openMenu, closeMenu, Menu } = useContextMenu();
  const handleContextMenu = useStableCallback((e: React.MouseEvent) => {
    e.preventDefault();
    openMenu(e);
  });

  const handleMarkReadClick = useStableCallback(() => {
    chatActions.resetTopicUnreadCount(topic);
    closeMenu();
  });

  const handleOpenPopoutClick = useStableCallback(() => {
    chatActions.openTopicPopout(topic);
    closeMenu();
  });

  const handleCloseClick = useStableCallback(() => {
    chatActions.closeTopic(topic);
    closeMenu();
  });

  const className = clsx({
    "room_carousel__item": true,
    [`room_carousel__item--${color}`]: true,
    [`room_carousel__item--selected`]: selected,
  });

  return (
    <>
      <div ref={ref} className={className} onClick={handleClick} onContextMenu={handleContextMenu}>
        {label?.substring(0, 2)}
        {unreadCount > 0 && <Badge count={unreadCount} max={100} />}
      </div>
      <Menu>
        <MenuItem onClick={handleMarkReadClick}>mark as read</MenuItem>
        <MenuItem onClick={handleOpenPopoutClick}>open mini chat</MenuItem>
        <MenuItem onClick={handleCloseClick}>close</MenuItem>
      </Menu>
    </>
  );
};

const SWIPER_FREE_MODE_OPTIONS = {
  enabled: true,
  sticky: true,
};

export interface RoomCarouselProps {
  className?: string;
  onChange: (topic: ThreadProviderProps) => void;
}

const RoomCarousel: React.FC<RoomCarouselProps> = ({ className, onChange }) => {
  const [{ rooms, whispers, mainTopics, mainActiveTopic }] = useChat();
  const ref = useRef<HTMLDivElement>();
  const size = useSize(ref);

  const slidesPerView = size ? Math.floor(size?.width / 52) : 1;

  const slides = mainTopics.map((topic) => {
    const key = Base64.fromUint8Array(topic.topicKey, true);
    const thread: ThreadState = topic.type === "ROOM" ? rooms.get(key) : whispers.get(key);

    return (
      <SwiperSlide key={key}>
        <RoomCarouselGem
          {...topic}
          color="green"
          label={thread.label}
          unreadCount={thread.unreadCount}
          selected={isEqual(topic, mainActiveTopic)}
          onChange={onChange}
        />
      </SwiperSlide>
    );
  });

  const handlePointerDown = useCallback((e: React.PointerEvent<HTMLDivElement>) => {
    if (DEVICE_TYPE === DeviceType.Portable) {
      e.stopPropagation();
    }
  }, []);

  return (
    <div
      className={clsx(className, "room_carousel")}
      ref={ref}
      onPointerDownCapture={handlePointerDown}
    >
      <Swiper
        slidesPerView={slidesPerView}
        spaceBetween={4}
        loop={slides.length > slidesPerView}
        mousewheel={true}
        modules={[Mousewheel, FreeMode]}
        freeMode={DEVICE_TYPE === DeviceType.Portable ? false : SWIPER_FREE_MODE_OPTIONS}
        touchStartPreventDefault={false}
      >
        {slides}
      </Swiper>
    </div>
  );
};

export default RoomCarousel;
