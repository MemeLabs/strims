// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./EmoteDetails.scss";

import React, { useRef } from "react";
import { FiExternalLink } from "react-icons/fi";
import { MdClose } from "react-icons/md";
import usePortal from "use-portal";

import { useRoom } from "../../contexts/Chat";
import { useLayout } from "../../contexts/Layout";
import useClickAway from "../../hooks/useClickAway";
import ExternalLink from "../ExternalLink";
import Emote from "./Emote";

interface EmoteDetailsProps {
  name: string;
  anchor: [number, number];
  onClose: () => void;
}

const EmoteDetails: React.FC<EmoteDetailsProps> = ({ name, anchor, onClose }) => {
  const layout = useLayout();
  const { Portal } = usePortal({ target: layout.root });

  const ref = useRef<HTMLDivElement>();
  useClickAway(ref, onClose);

  const [{ liveEmotes }] = useRoom();
  const emote = liveEmotes.find((e) => e.name === name);

  return (
    <Portal>
      <div
        className="emote_details"
        style={{
          "--menu-x": `${anchor[0]}px`,
          "--menu-y": `${anchor[1]}px`,
        }}
        ref={ref}
      >
        <button className="emote_details__close" onClick={onClose}>
          <MdClose />
        </button>
        <Emote name={name} shouldAnimateForever={true}>
          {name}
        </Emote>
        <div className="emote_details__label">
          <span className="emote_details__name">{emote.name}</span>
          {emote.contributor && (
            <div className="emote_details__contributor">
              <span className="emote_details__contributor__label">by:</span>
              <span className="emote_details__contributor__name">
                {emote.contributor.name}
              </span>
              {emote.contributor.link && (
                <ExternalLink
                  className="emote_details__contributor__link"
                  href={emote.contributor.link}
                >
                  <FiExternalLink />
                </ExternalLink>
              )}
            </div>
          )}
        </div>
      </div>
    </Portal>
  );
};

export default EmoteDetails;
