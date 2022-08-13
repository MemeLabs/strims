// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { ComponentProps } from "react";

import monkey from "../../../assets/directory/monkey.png";
import { ListingSnippetImage } from "../../apis/strims/network/v1/directory/directory";
import { useImage } from "../../hooks/useImage";

interface SnippetImageProps extends ComponentProps<"img"> {
  fallback?: string;
  source: ListingSnippetImage;
}

const SnippetImage: React.FC<SnippetImageProps> = ({ fallback = monkey, source, ...imgProps }) => {
  let url = "";
  switch (source?.sourceOneof?.case) {
    case ListingSnippetImage.SourceOneofCase.URL:
      url = source.sourceOneof.url;
      break;
    case ListingSnippetImage.SourceOneofCase.IMAGE:
      url = useImage(source.sourceOneof.image);
      break;
  }

  return <img src={url || fallback} {...imgProps} />;
};

export default SnippetImage;
