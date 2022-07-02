// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";
import React, { useMemo } from "react";
import { useParams } from "react-router-dom";

import { ListingContentType } from "../apis/strims/network/v1/directory/directory";
import DirectoryGrid, { DirectoryListing } from "../components/Directory/Grid";
import { useClient } from "../contexts/FrontendApi";
import { useListings } from "../hooks/directory";

const Directory: React.FC = () => {
  const params = useParams<"networkKey">();
  const client = useClient();

  const listings = useListings(
    useMemo(
      () => ({
        networkKeys: [Base64.toUint8Array(params.networkKey)],
        contentTypes: [
          ListingContentType.LISTING_CONTENT_TYPE_MEDIA,
          ListingContentType.LISTING_CONTENT_TYPE_EMBED,
        ],
      }),
      [params.networkKey]
    )
  );

  const handleTestClick = async () => {
    const networkKey = Base64.toUint8Array(params.networkKey);
    const res = await client.directory.test({ networkKey });
    console.log(res);
  };

  const gridListings = useMemo(() => {
    const gridListings: DirectoryListing[] = [];
    for (const [, n] of listings.networkListings) {
      for (const [, l] of n.listings) {
        gridListings.push({
          id: l.id,
          listing: l.listing,
          snippet: l.snippet,
          userCount: l.userCount,
        });
      }
    }
    return gridListings;
  }, [listings]);

  return (
    <div>
      <button onClick={handleTestClick} className="input input_button">
        test
      </button>
      <DirectoryGrid listings={gridListings} networkKey={params.networkKey} />
    </div>
  );
};

export default Directory;
