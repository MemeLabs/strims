// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Directory.stories.scss";

import React, { useEffect, useState } from "react";

import { ListingSnippetImage } from "../../../../apis/strims/network/v1/directory/directory";
import { ImageType } from "../../../../apis/strims/type/image";
import SnippetImage from "../../../../components/Directory/SnippetImage";
import { Story, fixtures } from "../../../storybook";

const urlSource = (url: string) => new ListingSnippetImage({ sourceOneof: { url } });

// An image carried as data, as a listing's channel logo may be.
const ImageData: React.FC = () => {
  const [source, setSource] = useState<ListingSnippetImage>(null);
  useEffect(() => {
    void fixtures.imageData(fixtures.iconUrls[3]).then((data) =>
      setSource(
        new ListingSnippetImage({
          sourceOneof: { image: { type: ImageType.IMAGE_TYPE_PNG, data } },
        })
      )
    );
  }, []);
  return source ? <SnippetImage source={source} /> : null;
};

const Figure: React.FC<{ caption: string; children: React.ReactNode }> = ({
  caption,
  children,
}) => (
  <div className="directory_story__images">
    <figure>
      {children}
      <figcaption>{caption}</figcaption>
    </figure>
  </div>
);

const stories: Story[] = [
  {
    name: "URL",
    component: () => (
      <Figure caption="a URL">
        <SnippetImage source={urlSource(fixtures.thumbnailUrls[0])} />
      </Figure>
    ),
  },
  {
    name: "Image data",
    component: () => (
      <Figure caption="image data">
        <ImageData />
      </Figure>
    ),
  },
  {
    name: "Fallback",
    component: () => (
      <Figure caption="no source: the default fallback">
        <SnippetImage source={undefined} />
      </Figure>
    ),
  },
  {
    name: "Empty URL",
    component: () => (
      <Figure caption="an empty URL: the fallback">
        <SnippetImage source={urlSource("")} />
      </Figure>
    ),
  },
  {
    name: "Custom fallback",
    component: () => (
      <Figure caption="no source, a fallback given">
        <SnippetImage source={undefined} fallback={fixtures.iconUrls[0]} />
      </Figure>
    ),
  },
];

export default stories;
