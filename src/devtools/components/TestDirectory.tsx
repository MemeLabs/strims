import React from "react";

import { Listing, ListingSnippet } from "../../apis/strims/network/v1/directory/directory";
import { DirectoryGrid } from "../../pages/Directory";

const Grid: React.FC = () => {
  const listing = {
    id: BigInt(1),
    listing: new Listing({
      "content": {
        "embed": {
          "service": 1,
          "id": "t4tv",
        },
      },
    }),
    snippet: new ListingSnippet({
      "title": "sportsball",
      "description": "",
      "tags": [],
      "category": "",
      "channelName": "t4tv",
      "viewerCount": BigInt(58),
      "live": true,
      "isMature": false,
      "thumbnail": {
        "sourceOneof": {
          "case": 1001,
          "url": "https://thumbnail.angelthump.com/thumbnails/t4tv.jpeg",
        },
      },
      "channelLogo": {
        "sourceOneof": {
          "case": 1001,
          "url":
            "https://images-angelthump.nyc3.cdn.digitaloceanspaces.com/profile-logos/5da062ce617b5de385e847fdbfc433f91fdb8f98405eb7a818c24e7a96b71509.png",
        },
      },
    }),
    viewerCount: 0,
  };

  const listings = [];
  for (let i = 0; i < 10; i++) {
    listings.push({ ...listing, id: BigInt(i) });
  }

  return <DirectoryGrid networkKey="" listings={listings} />;
};

export default Grid;
