import { Base64 } from "js-base64";
import * as React from "react";
import { useLocation, useParams } from "react-router-dom";

import { MainLayout } from "../components/MainLayout";
import VideoPlayer from "../components/VideoPlayer";
import { useClient } from "../contexts/FrontendApi";
import useQuery from "../hooks/useQuery";

interface PlayerTestRouteParams {
  networkKey: string;
}

interface PlayerTestQueryParams {
  swarmUri: string;
  mimeType: string;
}

const PlayerTest = () => {
  const params = useParams<PlayerTestRouteParams>();
  const query = useQuery<PlayerTestQueryParams>(useLocation().search);

  return (
    <MainLayout>
      <main className="home_page__main">
        <header className="home_page__subheader"></header>
        <section className="home_page__main__video">
          <VideoPlayer
            networkKey={Base64.toUint8Array(params.networkKey)}
            swarmUri={query.swarmUri}
            mimeType={query.mimeType}
          />
        </section>
      </main>
      <aside className="home_page__right">
        <header className="home_page__subheader"></header>
        <header className="home_page__chat__promo"></header>
        <div className="home_page__chat chat"></div>
      </aside>
    </MainLayout>
  );
};

export default PlayerTest;
