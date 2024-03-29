// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Search.scss";

import clsx from "clsx";
import escapeStringRegexp from "escape-string-regexp";
import { Base64 } from "js-base64";
import React, { RefObject, useEffect, useMemo, useRef, useState } from "react";
import Scrollbars from "react-custom-scrollbars-2";
import { Trans, useTranslation } from "react-i18next";
import { FiSearch } from "react-icons/fi";
import { useToggle } from "react-use";
import { Key } from "ts-key-enum";

import imgAngelThump from "../../../assets/directory/angelthump.png";
import imgTwitch from "../../../assets/directory/twitch.png";
import imgYouTube from "../../../assets/directory/youtube.png";
import {
  Listing,
  ListingContentType,
  NetworkListingsItem,
} from "../../apis/strims/network/v1/directory/directory";
import { useLazyCall } from "../../contexts/FrontendApi";
import { useOpenListing } from "../../hooks/directory";
import useClickAway from "../../hooks/useClickAway";
import { useStableCallback } from "../../hooks/useStableCallback";
import { createEmbedFromURL } from "../../lib/directory";
import SnippetImage from "../Directory/SnippetImage";

interface EmbedMenuItemProps {
  embed: Listing.IEmbed;
  selected: boolean;
  onMouseEnter: () => void;
  onSelect: () => void;
}

const EmbedMenuItem: React.FC<EmbedMenuItemProps> = ({
  embed,
  selected,
  onMouseEnter,
  onSelect,
}) => {
  const { t } = useTranslation();

  const label = (() => {
    switch (embed.service) {
      case Listing.Embed.Service.DIRECTORY_LISTING_EMBED_SERVICE_ANGELTHUMP:
        return {
          logo: imgAngelThump,
          name: t("directory.embedType.AngelThump"),
        };
      case Listing.Embed.Service.DIRECTORY_LISTING_EMBED_SERVICE_TWITCH_STREAM:
        return {
          logo: imgTwitch,
          name: t("directory.embedType.Twitch"),
        };
      case Listing.Embed.Service.DIRECTORY_LISTING_EMBED_SERVICE_TWITCH_VOD:
        return {
          logo: imgTwitch,
          name: t("directory.embedType.Twitch VOD"),
        };
      case Listing.Embed.Service.DIRECTORY_LISTING_EMBED_SERVICE_YOUTUBE:
        return {
          logo: imgYouTube,
          name: t("directory.embedType.YouTube"),
        };
    }
  })();

  return (
    <div
      className={clsx({
        "search__menu__embed": true,
        "search__menu__embed--selected": selected,
      })}
      onMouseEnter={onMouseEnter}
      onClick={onSelect}
    >
      <img className="search__menu__embed__logo" src={label.logo} />
      {t("directory.Embed")} {label.name}
    </div>
  );
};

interface ListingMenuItemProps {
  listing: NetworkListingsItem;
  selected: boolean;
  onMouseEnter: () => void;
  onSelect: () => void;
}

const ListingMenuItem: React.FC<ListingMenuItemProps> = ({
  listing,
  selected,
  onMouseEnter,
  onSelect,
}) => {
  const { snippet } = listing;
  const title = snippet.title.trim();

  return (
    <div
      className={clsx({
        "search__menu__channel": true,
        "search__menu__channel--selected": selected,
      })}
      onClick={onSelect}
      onMouseEnter={onMouseEnter}
    >
      <SnippetImage className="search__menu__channel__logo" source={snippet.channelLogo} />
      <div className="search__menu__channel__label">
        {title && (
          <span className="directory_grid__item__channel__title" title={title}>
            {title}
          </span>
        )}
        {snippet.channelName && (
          <span className="search__menu__channel__name">{snippet.channelName}</span>
        )}
      </div>
    </div>
  );
};

type SearchResult =
  | {
      type: "EMBED";
      embed: Listing.IEmbed;
      onSelect: () => void;
    }
  | {
      type: "LISTING";
      listing: NetworkListingsItem;
      onSelect: () => void;
    };

interface SearchMenuProps {
  results: SearchResult[];
  selectedIndex: number;
  scroll;
  onMouseEnter: (i: number) => void;
  onMouseLeave: () => void;
}

const SearchMenu: React.FC<SearchMenuProps> = ({
  results,
  selectedIndex,
  scroll,
  onMouseEnter,
  onMouseLeave,
}) => {
  let body = (
    <>
      {results.map((result, i) => {
        switch (result.type) {
          case "EMBED":
            return (
              <EmbedMenuItem
                key={i}
                selected={selectedIndex === i}
                onMouseEnter={() => onMouseEnter(i)}
                {...result}
              />
            );
          case "LISTING":
            return (
              <ListingMenuItem
                key={i}
                selected={selectedIndex === i}
                onMouseEnter={() => onMouseEnter(i)}
                {...result}
              />
            );
        }
      })}
    </>
  );

  if (scroll) {
    body = <Scrollbars autoHide>{body}</Scrollbars>;
  }

  return (
    <div className="search__menu" onMouseLeave={onMouseLeave}>
      {body}
    </div>
  );
};

interface SearchProps {
  menuOpen?: boolean;
  maxResults?: number;
  scrollMenu?: boolean;
  showCancel?: boolean;
  onDone?: () => void;
  inputRef?: RefObject<HTMLInputElement>;
}

const Search: React.FC<SearchProps> = ({
  menuOpen: forceMenuOpen = false,
  maxResults = Infinity,
  scrollMenu = false,
  showCancel = false,
  onDone,
  inputRef,
}) => {
  const { t } = useTranslation();

  const ref = useRef<HTMLDivElement>(null);
  const [isFocused, toggleIsFocused] = useToggle(false);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [query, setQuery] = useState("");
  const [getListingsRes, getListings] = useLazyCall("directory", "getListings");

  const results = useMemo<SearchResult[]>(() => {
    if (query === "") {
      return [];
    }

    const results: SearchResult[] = [];

    const embed = createEmbedFromURL(query);
    if (embed) {
      return [
        {
          type: "EMBED",
          embed,
          onSelect: () => selectEmbed(embed),
        },
      ];
    }

    const pattern = new RegExp(escapeStringRegexp(query), "i");

    for (const { network, listings } of getListingsRes.value?.listings || []) {
      for (const listing of listings) {
        if (
          pattern.exec(listing.snippet?.title) !== null ||
          pattern.exec(listing.snippet?.channelName) !== null
        ) {
          results.push({
            type: "LISTING",
            listing: listing,
            onSelect: () => selectListing(network.key, listing.listing),
          });
        }
      }
    }

    return results.slice(0, maxResults);
  }, [query, getListingsRes]);

  const menuOpen = forceMenuOpen || (isFocused && results.length !== 0);

  useEffect(() => setSelectedIndex(-1), [results.length, query]);

  useClickAway(ref, () => toggleIsFocused(false));

  const handleChange: React.ChangeEventHandler<HTMLInputElement> = useStableCallback((e) => {
    setQuery(e.target.value);
  });

  const handleKeyDown: React.KeyboardEventHandler<HTMLInputElement> = useStableCallback((e) => {
    switch (e.key) {
      case Key.Tab:
      case Key.ArrowDown:
        e.preventDefault();
        setSelectedIndex((i) => (i + 1) % results.length);
        return;
      case Key.ArrowUp:
        e.preventDefault();
        setSelectedIndex((i) => (results.length + i - 1) % results.length);
        return;
      case Key.Enter:
        e.preventDefault();
        results[Math.max(selectedIndex, 0)]?.onSelect();
        return;
      case Key.Escape:
        e.preventDefault();
        e.currentTarget.blur();
        return;
    }
  });

  const handleFocus = useStableCallback(() => {
    toggleIsFocused(true);
    void getListings({
      contentTypes: [
        ListingContentType.LISTING_CONTENT_TYPE_EMBED,
        ListingContentType.LISTING_CONTENT_TYPE_MEDIA,
      ],
    });
  });

  const selectEmbed = (embed: Listing.IEmbed) => {
    // TODO: this blows up if there are no directories loaded... select input? checkboxes?
    //       update the directories in all networks?
    const [{ network }] = getListingsRes.value.listings;
    selectListing(network.key, new Listing({ content: { embed } }));
    onDone?.();
  };

  const openListing = useOpenListing();
  const selectListing = (networkKey: Uint8Array, listing: Listing) => {
    openListing(Base64.fromUint8Array(networkKey, true), listing);

    setQuery("");
    onDone?.();
  };

  let box = (
    <div className="search__box">
      <form action="." className="search__form">
        <input
          ref={inputRef}
          type="search"
          className="search__input"
          autoCapitalize="off"
          autoComplete="off"
          spellCheck="false"
          value={query}
          onChange={handleChange}
          onKeyDown={handleKeyDown}
          onFocus={handleFocus}
          placeholder={t("directory.Search")}
        />
      </form>
      <FiSearch className="search__icon" />
    </div>
  );

  if (showCancel) {
    box = (
      <div className="search__base">
        {box}
        <button className="search__cancel" onClick={onDone}>
          <Trans>Cancel</Trans>
        </button>
      </div>
    );
  }

  return (
    <div
      className={clsx({
        "search": true,
        "search--menu_open": menuOpen,
      })}
      ref={ref}
    >
      {box}
      {menuOpen && (
        <SearchMenu
          selectedIndex={selectedIndex}
          results={results}
          scroll={scrollMenu}
          onMouseEnter={(i) => setSelectedIndex(i)}
          onMouseLeave={() => setSelectedIndex(-1)}
        />
      )}
    </div>
  );
};

export default Search;
