import "./Search.scss";

import clsx from "clsx";
import escapeStringRegexp from "escape-string-regexp";
import { Base64 } from "js-base64";
import React, { RefObject, useContext, useEffect, useRef, useState } from "react";
import Scrollbars from "react-custom-scrollbars-2";
import { Trans, useTranslation } from "react-i18next";
import { FiSearch } from "react-icons/fi";
import { useNavigate } from "react-router";
import { useToggle } from "react-use";
import { Key } from "ts-key-enum";

import imgAngelThump from "../../../assets/directory/angelthump.png";
import imgTwitch from "../../../assets/directory/twitch.png";
import imgYouTube from "../../../assets/directory/youtube.png";
import { Listing } from "../../apis/strims/network/v1/directory/directory";
import { DirectoryContext, DirectoryListing } from "../../contexts/Directory";
import { useLayout } from "../../contexts/Layout";
import { PlayerContext, PlayerMode } from "../../contexts/Player";
import useClickAway from "../../hooks/useClickAway";
import { formatUri, getListingPlayerSource } from "../../lib/directory";
import { DEVICE_TYPE, DeviceType } from "../../lib/userAgent";
import SnippetImage from "../Directory/SnippetImage";

const EMBED_COMMON_ID = "([\\w-]{1,30})";
const EMBED_URLS = [
  {
    pattern: new RegExp(`twitch\\.tv/videos/${EMBED_COMMON_ID}(?:.*&t=([^&$]+))`),
    embed: (v: RegExpExecArray) => ({
      service: Listing.Embed.Service.DIRECTORY_LISTING_EMBED_SERVICE_TWITCH_VOD,
      id: v[1],
      queryParams: v[2] ? { t: v[2] } : {},
    }),
  },
  {
    pattern: new RegExp(`twitch\\.tv/${EMBED_COMMON_ID}/?$`),
    embed: (v: RegExpExecArray) => ({
      service: Listing.Embed.Service.DIRECTORY_LISTING_EMBED_SERVICE_TWITCH_STREAM,
      id: v[1],
    }),
  },
  {
    pattern: new RegExp(`angelthump\\.com/(?:embed/)?${EMBED_COMMON_ID}$`),
    embed: (v: RegExpExecArray) => ({
      service: Listing.Embed.Service.DIRECTORY_LISTING_EMBED_SERVICE_ANGELTHUMP,
      id: v[1],
    }),
  },
  {
    pattern: new RegExp(`player\\.angelthump\\.com/.*?[&?]channel=${EMBED_COMMON_ID}`),
    embed: (v: RegExpExecArray) => ({
      service: Listing.Embed.Service.DIRECTORY_LISTING_EMBED_SERVICE_ANGELTHUMP,
      id: v[1],
    }),
  },
  {
    pattern: new RegExp(`youtube\\.com/watch.*?[&?]v=${EMBED_COMMON_ID}(?:.*&t=([^&$]+))`),
    embed: (v: RegExpExecArray) => ({
      service: Listing.Embed.Service.DIRECTORY_LISTING_EMBED_SERVICE_YOUTUBE,
      id: v[1],
      queryParams: v[2] ? { t: v[2] } : {},
    }),
  },
  {
    pattern: new RegExp(`youtu\\.be/${EMBED_COMMON_ID}(?:.*&t=([^&$]+))`),
    embed: (v: RegExpExecArray) => ({
      service: Listing.Embed.Service.DIRECTORY_LISTING_EMBED_SERVICE_YOUTUBE,
      id: v[1],
      queryParams: v[2] ? { t: v[2] } : {},
    }),
  },
  {
    pattern: new RegExp(`youtube\\.com/embed/${EMBED_COMMON_ID}(?:.*&t=([^&$]+))`),
    embed: (v: RegExpExecArray) => ({
      service: Listing.Embed.Service.DIRECTORY_LISTING_EMBED_SERVICE_YOUTUBE,
      id: v[1],
      queryParams: v[2] ? { t: v[2] } : {},
    }),
  },
];

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
  listing: DirectoryListing;
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
      listing: DirectoryListing;
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
  const [directory] = useContext(DirectoryContext);
  const [results, setResults] = useState<SearchResult[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [query, setQuery] = useState("");

  const menuOpen = forceMenuOpen || (isFocused && results.length !== 0);

  useClickAway(ref, () => toggleIsFocused(false));

  useEffect(() => {
    if (query === "") {
      setResults([]);
      return;
    }

    const results: SearchResult[] = [];

    for (const { pattern, embed } of EMBED_URLS) {
      const match = pattern.exec(query);
      if (match) {
        setResults([]);
        setResults([
          {
            type: "EMBED",
            embed: embed(match),
            onSelect: () => selectEmbed(embed(match)),
          },
        ]);
        return;
      }
    }

    const pattern = new RegExp(escapeStringRegexp(query), "i");

    for (const { networkKey, listings } of Object.values(directory)) {
      for (const listing of listings.values()) {
        if (
          pattern.exec(listing.snippet?.title) !== null ||
          pattern.exec(listing.snippet?.channelName) !== null
        ) {
          results.push({
            type: "LISTING",
            listing,
            onSelect: () => selectListing(networkKey, listing.listing),
          });
        }
      }
    }

    setResults(results.slice(0, maxResults));
  }, [query, directory]);

  useEffect(() => setSelectedIndex(-1), [results.length, query]);

  const handleKeyDown: React.KeyboardEventHandler<HTMLInputElement> = (e) => {
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
  };

  const navigate = useNavigate();
  const player = useContext(PlayerContext);
  const layout = useLayout();

  const selectEmbed = (embed: Listing.IEmbed) => {
    // TODO: this blows up if there are no directories loaded... select input? checkboxes?
    const [{ networkKey }] = Object.values(directory);
    selectListing(networkKey, new Listing({ content: { embed } }));
    onDone?.();
  };

  // TODO: DRY with grid (useDirectory?)
  const selectListing = (networkKey: Uint8Array, listing: Listing) => {
    layout.toggleOverlayOpen(true);
    layout.toggleShowVideo(true);
    player.setMode(PlayerMode.FULL);
    player.setSource(getListingPlayerSource(Base64.fromUint8Array(networkKey, true), listing));
    if (DEVICE_TYPE !== DeviceType.Portable) {
      const path = formatUri(Base64.fromUint8Array(networkKey, true), listing);
      player.setPath(path);
      navigate(path);
    }

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
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={handleKeyDown}
          onFocus={() => toggleIsFocused(true)}
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
