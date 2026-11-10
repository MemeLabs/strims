// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import {
  FrontendWatchListingsResponse,
  IListingSnippetImage,
  ListingSnippetImage,
} from "../apis/strims/network/v1/directory/directory";
import { resolveImages } from "./images";

const BASE = "https://node.example:8443";

const url = (u: string): IListingSnippetImage => ({ sourceOneof: { url: u } });

const snippetEvents = (
  ...snippets: { thumbnail?: IListingSnippetImage; channelLogo?: IListingSnippetImage }[]
) =>
  new FrontendWatchListingsResponse({
    events: [
      {
        event: {
          change: {
            listings: {
              network: { id: BigInt(1), name: "n", key: new Uint8Array([1]) },
              listings: snippets.map((snippet, i) => ({ id: BigInt(i + 1), snippet })),
            },
          },
        },
      },
    ],
  }).events;

const shown = (events: FrontendWatchListingsResponse.Event[]) =>
  events.flatMap(({ event }) =>
    event.case === FrontendWatchListingsResponse.Event.EventCase.CHANGE
      ? event.change.listings.listings.map(({ snippet }) =>
          [snippet.thumbnail, snippet.channelLogo].map((i) =>
            i?.sourceOneof.case === ListingSnippetImage.SourceOneofCase.URL
              ? i.sourceOneof.url
              : i
              ? "inline"
              : null
          )
        )
      : []
  );

describe("watch images", () => {
  it("points the API's paths at it and keeps other http(s) URLs", () => {
    const events = snippetEvents({
      thumbnail: url("/api/v1/directory/thumbnails/a/1/ff"),
      channelLogo: url("https://cdn.example/logo.png"),
    });
    assert.deepEqual(shown(resolveImages(events, BASE)), [
      [`${BASE}/api/v1/directory/thumbnails/a/1/ff`, "https://cdn.example/logo.png"],
    ]);
  });

  it("drops an image whose URL doesn't parse or isn't http(s), leaving the other listings", () => {
    const events = snippetEvents(
      { thumbnail: url("http://["), channelLogo: url("javascript:alert(1)") },
      { thumbnail: url("/api/v1/directory/thumbnails/a/2/ee") },
      { thumbnail: { sourceOneof: { image: { data: new Uint8Array([1]) } } } }
    );
    assert.deepEqual(shown(resolveImages(events, BASE)), [
      [null, null],
      [`${BASE}/api/v1/directory/thumbnails/a/2/ee`, null],
      ["inline", null],
    ]);
  });
});
