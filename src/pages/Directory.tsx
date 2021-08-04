/* eslint-disable no-console */

import { Base64 } from "js-base64";
import React, { useContext } from "react";
import { Link, useParams } from "react-router-dom";

import { DirectoryListing } from "../apis/strims/network/v1/directory";
import { DirectoryContext } from "../contexts/Directory";
import { useClient } from "../contexts/FrontendApi";

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
  // const [listings, dispatch] = React.useReducer(directoryReducer, []);
  const [directories] = useContext(DirectoryContext);
  const client = useClient();

  console.log(directories);

  const listings = directories[params.networkKey] || [];

  // React.useEffect(() => {
  //   const networkKey = Base64.toUint8Array(params.networkKey);
  //   const events = client.directory.open({ networkKey });
  //   events.on("data", ({ event }) => dispatch(event));
  //   events.on("close", () => console.log("directory event stream closed"));
  //   return () => events.destroy();
  // }, [params.networkKey]);

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
