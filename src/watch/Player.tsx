// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";
import React, { useCallback, useContext, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useLocation, useNavigate, useParams, useSearchParams } from "react-router";
import { useTitle } from "react-use";

import { Listing, ListingContentType } from "../apis/strims/network/v1/directory/directory";
import VideoPlayer from "../components/VideoPlayer";
import { useClient } from "../contexts/FrontendApi";
import { PlayerContext, Provider as PlayerProvider } from "../contexts/Player";
import { useSession } from "../contexts/Session";
import { useListings } from "../hooks/directory";
import { swarmId } from "../lib/media/renditions";
import Status from "./Status";
import { InvitationError, fetchInvitation } from "./api";
import { useBootstrapListings } from "./listings";
import { joinNetwork } from "./membership";
import { AnonymousSession } from "./session";

/** How long a refused watch of the node's own listings waits to ask again. */
const LISTINGS_RETRY_MS = 2000;

interface Stream {
  /** The network's key, base64url. */
  networkKey: string;
  keyBytes: Uint8Array;
  swarmUri: string;
  mimeType: string;
}

type Membership =
  | { status: "joining" }
  | { status: "member" }
  | { status: "failed"; error: Error; retryable: boolean };

// Waits `ms`, resolving early once `signal` aborts.
const sleep = (ms: number, signal: AbortSignal) =>
  new Promise<void>((resolve) => {
    const timer = setTimeout(resolve, ms);
    signal.addEventListener("abort", () => (clearTimeout(timer), resolve()), { once: true });
  });

/**
 * Joins the stream's network unless the node holds it already: a member as
 * soon as the node holds it, failed if the network's CA then refuses it.
 */
const useMembership = (key: Uint8Array): [Membership, () => void] => {
  const client = useClient();
  const [{ profile }] = useSession();
  const [membership, setMembership] = useState<Membership>({ status: "joining" });
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    const aborter = new AbortController();
    const { signal } = aborter;
    setMembership({ status: "joining" });
    joinNetwork(client.network, key, {
      invite: () => fetchInvitation(key, signal),
      name: profile?.name ?? "",
      onMember: () => setMembership({ status: "member" }),
      sleep: (ms) => sleep(ms, signal),
      signal,
    }).catch(
      (error: Error) =>
        !signal.aborted &&
        setMembership({
          status: "failed",
          error,
          retryable: !(error instanceof InvitationError) || error.retryable,
        })
    );
    return () => aborter.abort();
  }, [key, attempt]);

  return [membership, useCallback(() => setAttempt((n) => n + 1), [])];
};

// The page's way out of the player: back to the tiles it came from, or to
// the tiles when it was opened directly.
const useClose = () => {
  const navigate = useNavigate();
  const location = useLocation();
  return useCallback(() => {
    if (location.key !== "default") {
      void navigate(-1);
    } else {
      void navigate("/", { replace: true });
    }
  }, [location.key]);
};

/**
 * The stream on the node, once it's a member: the player starts at once,
 * and the listing opens (the node's viewership, and the renditions the
 * player picks from) once the node's own directory holds it.
 */
const StreamPlayer: React.FC<Stream & { onClose: () => void }> = ({
  networkKey,
  keyBytes,
  swarmUri,
  mimeType,
  onClose,
}) => {
  const { t } = useTranslation();
  const player = useContext(PlayerContext);
  // A network just joined starts in the node's directory a little later,
  // which refuses a watch naming it until then.
  const [watchAttempt, setWatchAttempt] = useState(0);
  const { networkListings, error } = useListings(
    useMemo(
      () => ({
        networkKeys: [keyBytes],
        contentTypes: [ListingContentType.LISTING_CONTENT_TYPE_MEDIA],
      }),
      [keyBytes, watchAttempt]
    )
  );
  useEffect(() => {
    if (!error) return;
    const timer = setTimeout(() => setWatchAttempt((n) => n + 1), LISTINGS_RETRY_MS);
    return () => clearTimeout(timer);
  }, [error]);

  const listed = useMemo(() => {
    const id = swarmId(swarmUri);
    for (const { listings } of networkListings.values()) {
      for (const l of listings.values()) {
        const { content } = l.listing ?? {};
        if (content?.case === Listing.ContentCase.MEDIA && swarmId(content.media.swarmUri) === id) {
          return l;
        }
      }
    }
    return null;
  }, [networkListings, swarmUri]);

  const isListed = !!listed;
  useEffect(() => {
    if (isListed) player.setSource({ type: "swarm", networkKey, swarmUri, mimeType });
  }, [isListed, networkKey, swarmUri, mimeType]);

  const bootstrap = useBootstrapListings();
  const title = useMemo(() => {
    if (listed?.snippet?.title) return listed.snippet.title;
    const id = swarmId(swarmUri);
    for (const { listings } of bootstrap.networkListings.values()) {
      for (const l of listings.values()) {
        const { content } = l.listing ?? {};
        if (content?.case === Listing.ContentCase.MEDIA && swarmId(content.media.swarmUri) === id) {
          return l.snippet?.title ?? "";
        }
      }
    }
    return "";
  }, [listed, bootstrap.networkListings, swarmUri]);
  useTitle(title.trim() || t("watch.title"));

  return (
    <div className="watch_player">
      <VideoPlayer
        networkKey={networkKey}
        swarmUri={swarmUri}
        mimeType={mimeType}
        handleClose={onClose}
      />
    </div>
  );
};

const Joined: React.FC<Stream & { onClose: () => void }> = (props) => {
  const { t } = useTranslation();
  const [membership, retry] = useMembership(props.keyBytes);

  switch (membership.status) {
    case "joining":
      return <Status message={t("watch.joining")} />;
    case "failed": {
      const offered = !(membership.error instanceof InvitationError) || membership.retryable;
      return (
        <Status
          message={offered ? t("watch.joinFailed") : t("watch.notOffered")}
          action={
            membership.retryable
              ? { label: t("watch.retry"), onClick: retry }
              : { label: t("watch.back"), onClick: props.onClose }
          }
        />
      );
    }
    case "member":
      return (
        <PlayerProvider>
          <StreamPlayer {...props} />
        </PlayerProvider>
      );
  }
};

interface PlayerPageProps {
  session: AnonymousSession;
}

/** A stream in the full window, joining its network first when it must. */
const PlayerPage: React.FC<PlayerPageProps> = ({ session }) => {
  const { t } = useTranslation();
  const onClose = useClose();
  const params = useParams<"networkKey">();
  const [search] = useSearchParams();
  const swarmUri = search.get("swarmUri");
  const mimeType = search.get("mimeType");
  const key = useMemo(() => {
    try {
      return Base64.toUint8Array(params.networkKey);
    } catch {
      return null;
    }
  }, [params.networkKey]);

  if (!key?.length || !swarmUri || !mimeType) {
    return (
      <Status
        message={t("watch.notOffered")}
        action={{ label: t("watch.back"), onClick: onClose }}
      />
    );
  }
  if (session.error) {
    return (
      <Status
        message={t("watch.startFailed")}
        action={{ label: t("watch.retry"), onClick: session.retry }}
      />
    );
  }
  if (!session.ready) {
    return <Status message={t("watch.starting")} />;
  }

  return (
    <Joined
      key={`${params.networkKey}:${swarmUri}`}
      networkKey={params.networkKey}
      keyBytes={key}
      swarmUri={swarmUri}
      mimeType={mimeType}
      onClose={onClose}
    />
  );
};

export default PlayerPage;
