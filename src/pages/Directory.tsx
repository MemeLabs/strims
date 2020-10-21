/* eslint-disable no-console */

import { Base64 } from "js-base64";
import * as React from "react";
import { Link, useParams } from "react-router-dom";

import { MainLayout } from "../components/MainLayout";
import { useClient, useLazyCall } from "../contexts/Api";
import { useProfile } from "../contexts/Profile";
import { useTheme } from "../contexts/Theme";
import { DirectoryServerEvent, IDirectoryListing } from "../lib/pb";

interface Listing {
  key: string;
  listing: IDirectoryListing;
}

const directoryReducer = (listings: Listing[], action: DirectoryServerEvent): Listing[] => {
  switch (action.body) {
    case "publish": {
      const listing: Listing = {
        key: Base64.fromUint8Array(action.publish.listing.key),
        listing: action.publish.listing,
      };
      return [listing, ...listings.filter((l) => l.key !== listing.key)];
    }
    case "unpublish": {
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

const Directory = () => {
  const params = useParams<DirectoryParams>();
  const [{ colorScheme }, { setColorScheme }] = useTheme();
  const [{ profile }, { clearProfile }] = useProfile();
  const [listings, dispatch] = React.useReducer(directoryReducer, []);
  const client = useClient();

  const networkKey = Base64.toUint8Array(params.networkKey);

  React.useEffect(() => {
    client.network.startVPN().on("data", (vpnEvent) => {
      if (vpnEvent.networkOpen) {
        const events = client.network.getDirectoryEvents({ networkKey });
        events.on("data", dispatch);
      }
    });
  }, []);

  const handleTestClick = async () => {
    const res = await client.network.testDirectoryPublish({ networkKey });
    console.log(res);
  };

  return (
    <MainLayout>
      <main className="home_page__main">
        <header className="home_page__subheader"></header>
        <section className="home_page__main__video">
          <div>
            <button onClick={handleTestClick} className="input input_button">
              test
            </button>
            {listings.map(({ key, listing }) => (
              <div key={key}>
                <Link to={`/player/${params.networkKey}/${key}`}>
                  <span>{listing.title}</span>
                </Link>
                <span>{listing.tags}</span>
              </div>
            ))}
          </div>
        </section>
      </main>
      <aside className="home_page__right">
        <header className="home_page__subheader"></header>
        <header className="home_page__chat__promo"></header>
        <div className="home_page__chat">chat</div>
      </aside>
    </MainLayout>
  );
};

export default Directory;
