// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "swiper/css";

import "./RoomCarousel.scss";

import { useDrag } from "@use-gesture/react";
import clsx from "clsx";
import { Base64 } from "js-base64";
import { isEqual } from "lodash";
import React, { MouseEventHandler, ReactNode, useCallback, useRef } from "react";
import { FreeMode, Mousewheel } from "swiper";
import { Swiper, SwiperSlide } from "swiper/react";

import { Image } from "../../apis/strims/type/image";
import { ThreadProviderProps, ThreadState, Topic, useChat } from "../../contexts/Chat";
import { useImage } from "../../hooks/useImage";
import useSize from "../../hooks/useSize";
import { useStableCallback } from "../../hooks/useStableCallback";
import { DEVICE_TYPE, DeviceType } from "../../lib/userAgent";
import Badge from "../Badge";
import { MenuItem, useContextMenu } from "../ContextMenu";

interface RoomCarouselGemProps extends Topic {
  color: string;
  label: string;
  icon: Image;
  unreadCount: number;
  onChange: (topic: Topic) => void;
  selected: boolean;
}

const RoomCarouselGem: React.FC<RoomCarouselGemProps> = (props) => {
  const { icon, unreadCount, type, topicKey, onChange, selected } = props;
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
    "room_carousel__item--selected": selected,
  });

  const Gem = icon ? RoomCarouselGemIcon : RoomCarouselGemFallback;

  return (
    <div className={className}>
      <Gem {...props} ref={ref} onClick={handleClick} onContextMenu={handleContextMenu}>
        {unreadCount > 0 && <Badge count={unreadCount} max={100} />}
      </Gem>
      <Menu>
        <MenuItem onClick={handleMarkReadClick}>mark as read</MenuItem>
        <MenuItem onClick={handleOpenPopoutClick}>open mini chat</MenuItem>
        <MenuItem onClick={handleCloseClick}>close</MenuItem>
      </Menu>
    </div>
  );
};

interface RoomCarouselGemImplProps extends RoomCarouselGemProps {
  children: ReactNode;
  onClick: MouseEventHandler<HTMLDivElement>;
  onContextMenu: MouseEventHandler<HTMLDivElement>;
}

const RoomCarouselGemFallback = React.forwardRef<HTMLDivElement, RoomCarouselGemImplProps>(
  ({ children, color, label, selected, onClick, onContextMenu }, ref) => {
    const className = clsx({
      "room_carousel__gem": true,
      [`room_carousel__gem--${color}`]: true,
      "room_carousel__gem--selected": selected,
    });

    return (
      <div ref={ref} className={className} onClick={onClick} onContextMenu={onContextMenu}>
        {label?.substring(0, 2)}
        {children}
      </div>
    );
  }
);

RoomCarouselGemFallback.displayName = "RoomCarouselGemFallback";

const RoomCarouselGemIcon = React.forwardRef<HTMLDivElement, RoomCarouselGemImplProps>(
  ({ children, icon, selected, onClick, onContextMenu }, ref) => {
    const className = clsx({
      "room_carousel__gem": true,
      "room_carousel__gem--with_icon": true,
      "room_carousel__gem--selected": selected,
    });
    const style = { "--chat-icon": `url(${useImage(icon)})` };

    return (
      <div
        ref={ref}
        className={className}
        style={style}
        onClick={onClick}
        onContextMenu={onContextMenu}
      >
        {children}
      </div>
    );
  }
);

RoomCarouselGemIcon.displayName = "RoomCarouselGemIcon";

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

  const slidesPerView = size ? Math.round(size?.width / 52) : 1;

  const slides = mainTopics.map((topic) => {
    const key = Base64.fromUint8Array(topic.topicKey, true);
    const thread: ThreadState = topic.type === "ROOM" ? rooms.get(key) : whispers.get(key);

    return (
      <SwiperSlide key={key}>
        <RoomCarouselGem
          {...topic}
          color="green"
          label={thread.label}
          icon={thread.icon}
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
        className="room_carousel__swiper"
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
