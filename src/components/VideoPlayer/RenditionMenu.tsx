// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./RenditionMenu.scss";

import React, { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { MdCheck, MdHighQuality } from "react-icons/md";

import useClickAway from "../../hooks/useClickAway";
import type { RenditionChoice } from "./useRendition";

type RenditionMenuProps = Pick<RenditionChoice, "options" | "playing" | "picked" | "pick">;

// The player controls' rendition menu: Auto, the choice's, naming the
// rendition playing, then each rendition the browser plays. Nothing for a
// stream of one.
const RenditionMenu: React.FC<RenditionMenuProps> = ({ options, playing, picked, pick }) => {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  useClickAway(ref, () => setOpen(false), { enable: open });
  if (!options.length) return null;

  const choose = (id: string | null) => {
    pick(id);
    setOpen(false);
  };
  const playingLabel = options.find((o) => o.id === playing)?.label;
  const item = (id: string | null, text: string) => (
    <li key={id ?? ""} role="none">
      <button role="menuitemradio" aria-checked={picked === id} onClick={() => choose(id)}>
        {picked === id ? <MdCheck /> : <span className="mark" />}
        {text}
      </button>
    </li>
  );

  return (
    <div className="video_player__rendition_menu" ref={ref}>
      <div className="button-wrap renditions">
        <button
          data-tip={t("player.Quality")}
          aria-haspopup="menu"
          aria-expanded={open}
          onClick={() => setOpen(!open)}
        >
          <MdHighQuality />
        </button>
      </div>
      {open && (
        <ul className="video_player__renditions" role="menu">
          {item(
            null,
            picked === null && playingLabel
              ? t("player.Auto playing", { rendition: playingLabel })
              : t("player.Auto")
          )}
          {options.map((o) => item(o.id, o.label))}
        </ul>
      )}
    </div>
  );
};

export default RenditionMenu;
