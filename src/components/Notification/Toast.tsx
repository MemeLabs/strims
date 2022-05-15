// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Toast.scss";

import clsx from "clsx";
import React, { useEffect, useLayoutEffect, useRef, useState } from "react";
import ReactDOM from "react-dom";
import { FiAlertCircle, FiAlertOctagon, FiAlertTriangle, FiCheckCircle } from "react-icons/fi";

import { Notification } from "../../apis/strims/notification/v1/notification";
import { useClient } from "../../contexts/FrontendApi";
import { useLayout } from "../../contexts/Layout";
import { useNotification } from "../../contexts/Notification";

interface ToastItemIconProps {
  status: Notification.Status;
}

const ToastItemIcon: React.FC<ToastItemIconProps> = ({ status }) => {
  const className = clsx(
    "notification_toast__item__icon",
    `notification_toast__item__icon--${Notification.Status[status].toLowerCase()}`
  );

  switch (status) {
    case Notification.Status.STATUS_INFO:
      return <FiAlertCircle className={className} />;
    case Notification.Status.STATUS_SUCCESS:
      return <FiCheckCircle className={className} />;
    case Notification.Status.STATUS_WARNING:
      return <FiAlertTriangle className={className} />;
    case Notification.Status.STATUS_ERROR:
      return <FiAlertOctagon className={className} />;
  }
};

interface ToastItemProps {
  closing: boolean;
  notification: Notification;
  onClick?: React.MouseEventHandler<HTMLDivElement>;
}

const ToastItem: React.FC<ToastItemProps> = ({ closing, notification, onClick }) => {
  const ref = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    const fid = requestAnimationFrame(() => {
      ref.current.classList.add("notification_toast__item--open");
    });
    return () => cancelAnimationFrame(fid);
  }, []);

  return (
    <div
      ref={ref}
      className={clsx(
        "notification_toast__item",
        `notification_toast__item--${Notification.Status[notification.status].toLowerCase()}`,
        { "notification_toast__item--closing": closing }
      )}
      onClick={onClick}
    >
      <ToastItemIcon status={notification.status} />
      <div className="notification_toast__item__title">{notification.title}</div>
      {notification.message && (
        <div className="notification_toast__item__title">{notification.message}</div>
      )}
    </div>
  );
};

interface ToastProps {
  timeoutMs?: number;
  closeDurationMs?: number;
}

const Toast: React.FC<ToastProps> = ({ timeoutMs = 5000, closeDurationMs = 500 }) => {
  const { root } = useLayout();
  const [items, setItems] = useState<ToastItemProps[]>([]);
  const { notifications } = useNotification();
  const client = useClient();

  const refreshItems = () => {
    const now = Date.now();
    const items: ToastItemProps[] = [];

    for (const notification of notifications) {
      const age = now - Number(notification.createdAt);
      if (age < timeoutMs + closeDurationMs) {
        items.push({
          closing: age - timeoutMs > 0,
          notification,
          onClick: () => client.notification.dismiss({ ids: [notification.id] }),
        });
      }
    }

    setItems(items);
  };

  useEffect(refreshItems, [notifications]);

  useEffect(() => {
    if (items.length > 0) {
      const timeout = timeoutMs - Date.now() + Number(items[0].notification.createdAt);
      const tid = setTimeout(refreshItems, timeout > 0 ? timeout : timeout + closeDurationMs);
      return () => clearTimeout(tid);
    }
  }, [items]);

  if (items.length === 0) {
    return null;
  }

  return ReactDOM.createPortal(
    <div className="notification_toast">
      {items.map((props) => (
        <ToastItem key={props.notification.id.toString()} {...props} />
      ))}
    </div>,
    root
  );
};

export default Toast;
