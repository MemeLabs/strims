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

import { RoomProviderProps, useChat } from "../../contexts/Chat";
import useSize from "../../hooks/useSize";
import { useStableCallback } from "../../hooks/useStableCallback";
import { DEVICE_TYPE, DeviceType } from "../../lib/userAgent";
import Badge from "../Badge";
import { MenuItem, useContextMenu } from "../ContextMenu";

const SWIPER_FREE_MODE_OPTIONS = {
  enabled: true,
  sticky: true,
};

interface RoomCarouselGemProps extends RoomProviderProps {
  color: string;
  label: string;
  onChange: (topic: RoomProviderProps) => void;
  selected: boolean;
}

const RoomCarouselGem: React.FC<RoomCarouselGemProps> = ({
  color,
  label,
  type,
  topicKey,
  onChange,
  selected,
}) => {
  const [, { openTopicPopout, closeTopic }] = useChat();

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
        closeTopic({ type, topicKey });
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

  const handleClick = useStableCallback(() => onChange({ type, topicKey }));

  const { openMenu, closeMenu, Menu } = useContextMenu();
  const handleContextMenu = useStableCallback((e: React.MouseEvent) => {
    e.preventDefault();
    openMenu(e);
  });

  const handleOpenPopoutClick = useStableCallback(() => {
    openTopicPopout({ type, topicKey });
    closeMenu();
  });

  const handleCloseClick = useStableCallback(() => {
    closeTopic({ type, topicKey });
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
        {label}
        <Badge count={33} max={100} />
      </div>
      <Menu>
        <MenuItem>mark as read</MenuItem>
        <MenuItem onClick={handleOpenPopoutClick}>open mini chat</MenuItem>
        <MenuItem onClick={handleCloseClick}>close</MenuItem>
      </Menu>
    </>
  );
};

export interface RoomCarouselProps {
  className?: string;
  onChange: (topic: RoomProviderProps) => void;
  selected: RoomProviderProps;
}

const RoomCarousel: React.FC<RoomCarouselProps> = ({ className, onChange, selected }) => {
  const handleTouchStart = useCallback((e: React.PointerEvent<HTMLDivElement>) => {
    if (DEVICE_TYPE === DeviceType.Portable) {
      e.stopPropagation();
    }
  }, []);

  const ref = useRef<HTMLDivElement>();
  const size = useSize(ref.current);

  const slidesPerView = size ? Math.floor(size?.width / 52) : 1;

  const [{ rooms, whispers, mainTopics, mainActiveTopic }] = useChat();
  const gems: RoomCarouselGemProps[] = mainTopics.map((topic) => {
    let label: string;
    switch (topic.type) {
      case "ROOM": {
        const room = rooms.get(Base64.fromUint8Array(topic.topicKey, true));
        label = room.room?.name.substring(0, 2) ?? "...";
        break;
      }
      case "WHISPER": {
        const whisper = whispers.get(Base64.fromUint8Array(topic.topicKey, true));
        label = whisper.thread?.alias.substring(0, 2) ?? "...";
        break;
      }
    }

    return {
      ...topic,
      color: "green",
      label,
      onChange,
      selected: isEqual(topic, mainActiveTopic),
    };
  });

  return (
    <div
      className={clsx(className, "room_carousel")}
      ref={ref}
      onPointerDownCapture={handleTouchStart}
    >
      <Swiper
        slidesPerView={slidesPerView}
        spaceBetween={4}
        loop={gems.length > slidesPerView}
        mousewheel={true}
        modules={[Mousewheel, FreeMode]}
        freeMode={DEVICE_TYPE === DeviceType.Portable ? false : SWIPER_FREE_MODE_OPTIONS}
        touchStartPreventDefault={false}
      >
        {gems.map((props, i) => (
          <SwiperSlide key={i}>
            <RoomCarouselGem {...props} />
          </SwiperSlide>
        ))}
      </Swiper>
    </div>
  );
};

export default RoomCarousel;
