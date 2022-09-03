// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { ComponentProps } from "react";

import monkey from "../../../assets/directory/monkey.png";
import { ListingSnippetImage } from "../../apis/strims/network/v1/directory/directory";
import { Image } from "../../apis/strims/type/image";
import { useImage } from "../../hooks/useImage";

interface SnippetImageProps extends ComponentProps<"img"> {
  fallback?: string;
  source: ListingSnippetImage;
}

const SnippetImage: React.FC<SnippetImageProps> = ({ fallback = monkey, source, ...imgProps }) => {
  switch (source?.sourceOneof?.case) {
    case ListingSnippetImage.SourceOneofCase.URL:
      return <img src={source.sourceOneof.url || fallback} {...imgProps} />;
    case ListingSnippetImage.SourceOneofCase.IMAGE:
      return <SnippetImageWithImage image={source.sourceOneof.image} {...imgProps} />;
    default:
      return <img src={fallback} {...imgProps} />;
  }
};

interface SnippetImageWithImageProps extends ComponentProps<"img"> {
  image: Image;
}

const SnippetImageWithImage: React.FC<SnippetImageWithImageProps> = ({ image, ...imgProps }) => (
  <img src={useImage(image)} {...imgProps} />
);

export default SnippetImage;
