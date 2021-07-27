/* eslint-disable no-console */

import { Base64 } from "js-base64";
import React from "react";
import { Link, useParams } from "react-router-dom";

import { DirectoryEvent, DirectoryListing } from "../apis/strims/network/v1/directory";
import { useClient } from "../contexts/FrontendApi";

interface Listing {
  key: string;
  listing: DirectoryListing;
}

const directoryReducer = (listings: Listing[], { body: action }: DirectoryEvent): Listing[] => {
  console.log(action);
  switch (action.case) {
    case DirectoryEvent.BodyCase.PUBLISH: {
      const listing: Listing = {
        key: Base64.fromUint8Array(action.publish.listing.key),
        listing: action.publish.listing,
      };
      return [listing, ...listings.filter((l) => l.key !== listing.key)];
    }
    case DirectoryEvent.BodyCase.UNPUBLISH: {
      const key = Base64.fromUint8Array(action.unpublish.key);
      return listings.filter((l) => l.key !== key);
    }
    default:
      return listings;
  }
};

interface DirectoryParams {
  networkKey: string;
}

const formatUri = (networkKey: string, { content }: DirectoryListing): string => {
  switch (content.case) {
    case DirectoryListing.ContentCase.MEDIA: {
      const mimeType = encodeURIComponent(content.media.mimeType);
      const swarmUri = encodeURIComponent(content.media.swarmUri);
      return `/player/${networkKey}?mimeType=${mimeType}&swarmUri=${swarmUri}`;
    }
    default:
      return "";
  }
};

const Directory: React.FC = () => {
  const params = useParams<DirectoryParams>();
  const [listings, dispatch] = React.useReducer(directoryReducer, []);
  const client = useClient();

  React.useEffect(() => {
    const networkKey = Base64.toUint8Array(params.networkKey);
    const events = client.directory.open({ networkKey });
    events.on("data", ({ event }) => dispatch(event));
    events.on("close", () => console.log("directory event stream closed"));
    return () => events.destroy();
  }, [params.networkKey]);

  const handleTestClick = async () => {
    const networkKey = Base64.toUint8Array(params.networkKey);
    const res = await client.directory.test({ networkKey });
    console.log(res);
  };

  return (
    <section className="home_page__main__video">
      <div>
        <button onClick={handleTestClick} className="input input_button">
          test
        </button>
        {listings.map(({ key, listing }) => (
          <div key={key}>
            <Link to={formatUri(params.networkKey, listing)}>
              <span>{listing.snippet.title}</span>
            </Link>
            {" | "}
            <span>{listing.snippet.tags}</span>
            {" | "}
            <span>{new Date(Number(listing.timestamp) * 1000).toLocaleString()}</span>
          </div>
        ))}
      </div>
    </section>
  );
};

export default Directory;
