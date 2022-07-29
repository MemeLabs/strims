// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import Host from "@memelabs/protobuf/lib/rpc/host";
import ServiceRegistry from "@memelabs/protobuf/lib/rpc/service";
import { Base64 } from "js-base64";
import React from "react";

import { FrontendClient } from "../../apis/client";
import { Listing, ListingSnippet, Network } from "../../apis/strims/network/v1/directory/directory";
import { registerDirectoryFrontendService } from "../../apis/strims/network/v1/directory/directory_rpc";
import DirectoryGrid, { DirectoryListing } from "../../components/Directory/Grid";
import Search from "../../components/Directory/Search";
import { Provider as ApiProvider } from "../../contexts/FrontendApi";
import { AsyncPassThrough } from "../../lib/stream";
import events from "../mocks/directory/events";
import DirectoryService from "../mocks/directory/service";

const SearchStory: React.FC = () => {
  const [[service, client]] = React.useState((): [DirectoryService, FrontendClient] => {
    const svc = new ServiceRegistry();
    const service = new DirectoryService();
    registerDirectoryFrontendService(svc, service);

    const [a, b] = [new AsyncPassThrough(), new AsyncPassThrough()];
    new Host(a, b, svc);
    return [service, new FrontendClient(b, a)];
  });

  return (
    <div className="directory_mockup">
      <ApiProvider value={client}>
        <div className="directory_mockup__header">
          <Search maxResults={10} />
        </div>
      </ApiProvider>
    </div>
  );
};

const GridStory: React.FC = () => {
  const listings: DirectoryListing[] = [];
  for (const e of events) {
    listings.push({
      id: e.id,
      listing: new Listing(e.listing),
      snippet: new ListingSnippet(e.snippet),
      userCount: 0,
      recentUserCount: 0,
      network: new Network({
        id: BigInt(0),
        name: "test-network",
        key: Base64.toUint8Array("cgqhekoCTcy7OOkRdbNbYG3J4svZorYlH3KKaT660BE="),
      }),
    });
  }

  return <DirectoryGrid listings={listings} />;
};

export default [
  {
    name: "Search",
    component: () => <SearchStory />,
  },
  {
    name: "Grid",
    component: () => <GridStory />,
  },
];
