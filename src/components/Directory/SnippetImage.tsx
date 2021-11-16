import React, { ComponentProps } from "react";

import monkey from "../../../assets/directory/monkey.png";
import { ListingSnippetImage } from "../../apis/strims/network/v1/directory/directory";
import { Image, ImageType } from "../../apis/strims/type/image";
import useObjectURL from "../../hooks/useObjectURL";

const toFileType = (t: ImageType) => {
  switch (t) {
    case ImageType.IMAGE_TYPE_APNG:
      return "image/apng";
    case ImageType.IMAGE_TYPE_AVIF:
      return "image/avif";
    case ImageType.IMAGE_TYPE_GIF:
      return "image/gif";
    case ImageType.IMAGE_TYPE_JPEG:
      return "image/jpeg";
    case ImageType.IMAGE_TYPE_PNG:
      return "image/png";
    case ImageType.IMAGE_TYPE_WEBP:
      return "image/webp";
  }
};

const useImage = (image: Image): string => useObjectURL(toFileType(image.type), image.data);

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
