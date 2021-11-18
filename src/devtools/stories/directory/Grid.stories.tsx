import React from "react";

import { Listing, ListingSnippet } from "../../../apis/strims/network/v1/directory/directory";
import DirectoryGrid from "../../../components/Directory/Grid";
import { DirectoryListing } from "../../../contexts/Directory";
import events from "../../mocks/directory/events";

const Grid: React.FC = () => {
  const listings: DirectoryListing[] = [];
  for (const e of events) {
    listings.push({
      id: e.id,
      listing: new Listing(e.listing),
      snippet: new ListingSnippet(e.snippet),
      viewerCount: 0,
    });
  }

  return <DirectoryGrid networkKey="" listings={listings} />;
};

export default [
  {
    name: "grid",
    component: () => <Grid />,
  },
];
