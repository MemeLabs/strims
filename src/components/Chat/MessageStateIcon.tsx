// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./MessageStateIcon.scss";

import React from "react";
import { useTranslation } from "react-i18next";
import { BiMessageSquareDots, BiMessageSquareError } from "react-icons/bi";

import { MessageState } from "../../apis/strims/chat/v1/chat";

interface MessageStateIconProps {
  messageState: MessageState;
}

const MessageStateIcon: React.FC<MessageStateIconProps> = ({ messageState }) => {
  const { t } = useTranslation();

  switch (messageState) {
    case MessageState.MESSAGE_STATE_ENQUEUED:
      return (
        <BiMessageSquareDots
          title={t("chat.Sending")}
          className="chat__message_state_icon chat__message_state_icon--enqueued"
        />
      );
    case MessageState.MESSAGE_STATE_FAILED:
      return (
        <BiMessageSquareError
          title={t("chat.Message delivery failed")}
          className="chat__message_state_icon chat__message_state_icon--failed"
        />
      );
    default:
      return null;
  }
};

export default MessageStateIcon;
