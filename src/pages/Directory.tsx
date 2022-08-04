// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";
import React, { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { useParams } from "react-router-dom";
import { useTitle } from "react-use";

import { ListingContentType } from "../apis/strims/network/v1/directory/directory";
import DirectoryGrid, { DirectoryListing } from "../components/Directory/Grid";
import { useClient } from "../contexts/FrontendApi";
import { useListings } from "../hooks/directory";
import first from "../lib/first";

const TestButton = () => {
  // TODO: feature gates
  if (IS_PRODUCTION) {
    return null;
  }

  const params = useParams<"networkKey">();

  const client = useClient();
  const handleTestClick = async () => {
    const networkKey = Base64.toUint8Array(params.networkKey);
    const res = await client.directory.test({ networkKey });
    console.log(res);
  };

  return (
    <button onClick={handleTestClick} className="input input_button">
      test
    </button>
  );
};

const Directory: React.FC = () => {
  const { t } = useTranslation();
  const params = useParams<"networkKey">();

  const listings = useListings(
    useMemo(
      () => ({
        networkKeys: params.networkKey ? [Base64.toUint8Array(params.networkKey)] : [],
        contentTypes: [
          ListingContentType.LISTING_CONTENT_TYPE_MEDIA,
          ListingContentType.LISTING_CONTENT_TYPE_EMBED,
        ],
      }),
      [params.networkKey]
    )
  );

  const networkName = params.networkKey && first(listings.networkListings.values())?.network.name;
  const title = networkName
    ? t("directory.title", { network: networkName })
    : t("directory.titleFallback");
  useTitle(title);

  const gridListings = useMemo(() => {
    const gridListings: DirectoryListing[] = [];
    for (const [, n] of listings.networkListings) {
      for (const [, l] of n.listings) {
        gridListings.push({
          id: l.id,
          listing: l.listing,
          snippet: l.snippet,
          userCount: l.userCount,
          recentUserCount: l.recentUserCount,
          network: n.network,
        });
      }
    }
    return gridListings.sort((a, b) => {
      const d = a.userCount - b.userCount;
      return d != 0 ? d : Number(a.id - b.id);
    });
  }, [listings]);

  return (
    <div>
      <TestButton />
      <DirectoryGrid listings={gridListings} />
    </div>
  );
};

export default Directory;
