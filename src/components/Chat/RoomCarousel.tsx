// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "swiper/css";

import "./RoomCarousel.scss";

import { useDrag } from "@use-gesture/react";
import clsx from "clsx";
import { isEqual } from "lodash";
import React, { useCallback, useRef } from "react";
import { FreeMode, Mousewheel } from "swiper";
import { Swiper, SwiperSlide } from "swiper/react";

import { RoomProviderProps, useChat } from "../../contexts/Chat";
import useSize from "../../hooks/useSize";
import { useStableCallback } from "../../hooks/useStableCallback";
import { DEVICE_TYPE, DeviceType } from "../../lib/userAgent";

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
  const [, { closeRoom, closeWhispers }] = useChat();

  const closeTopic = () => {
    switch (type) {
      case "ROOM":
        return closeRoom(topicKey);
      case "WHISPER":
        return closeWhispers(topicKey);
    }
  };

  const ref = useRef<HTMLDivElement>();
  useDrag(
    ({ movement: [, my], swipe: [, sy], dragging, first }) => {
      ref.current.style.setProperty("--drag-offset", `${my}px`);

      if (first && dragging) {
        ref.current.classList.add("room_carousel__item--dragging");
      } else if (!dragging) {
        ref.current.classList.remove("room_carousel__item--dragging");
        ref.current.style.removeProperty("--drag-offset");
      }

      if (sy === -1) {
        closeTopic();
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

  const handleContextMenu = useStableCallback((e: React.MouseEvent) => {
    e.preventDefault();
    closeTopic();
  });

  const className = clsx({
    "room_carousel__item": true,
    [`room_carousel__item--${color}`]: true,
    [`room_carousel__item--selected`]: selected,
  });

  return (
    <div ref={ref} className={className} onClick={handleClick} onContextMenu={handleContextMenu}>
      {label}
    </div>
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

  const gems: RoomCarouselGemProps[] = [];

  const [{ rooms, whispers }] = useChat();
  for (const room of rooms.values()) {
    gems.push({
      color: "green",
      label: room.room?.name.substring(0, 2) ?? "...",
      type: "ROOM",
      topicKey: room.serverKey,
      onChange,
      selected: selected?.type === "ROOM" && isEqual(selected?.topicKey, room.serverKey),
    });
  }
  for (const whisper of whispers.values()) {
    gems.push({
      color: "green",
      label: whisper.thread?.alias.substring(0, 2) ?? "...",
      type: "WHISPER",
      topicKey: whisper.peerKey,
      onChange,
      selected: selected?.type === "WHISPER" && isEqual(selected?.topicKey, whisper.peerKey),
    });
  }

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
