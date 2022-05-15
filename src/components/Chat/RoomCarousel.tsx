// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "swiper/css";

import "./RoomCarousel.scss";

import clsx from "clsx";
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
}

const RoomCarouselGem: React.FC<RoomCarouselGemProps> = ({
  color,
  label,
  type,
  topicKey,
  onChange,
}) => {
  const handleClick = useStableCallback(() => onChange({ type, topicKey }));

  return (
    <div className={`room_carousel__item room_carousel__item--${color}`} onClick={handleClick}>
      {label}
    </div>
  );
};

export interface RoomCarouselProps {
  className: string;
  onChange: (topic: RoomProviderProps) => void;
}

const RoomCarousel: React.FC<RoomCarouselProps> = ({ className, onChange }) => {
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
    });
  }
  for (const whisper of whispers.values()) {
    gems.push({
      color: "green",
      label: whisper.thread?.alias.substring(0, 2) ?? "...",
      type: "WHISPER",
      topicKey: whisper.peerKey,
      onChange,
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
        nested={true}
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
