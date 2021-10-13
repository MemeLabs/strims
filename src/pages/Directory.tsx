/* eslint-disable no-console */

import clsx from "clsx";
import { Base64 } from "js-base64";
import React, { ComponentProps, useContext, useEffect } from "react";
import { Link, useParams } from "react-router-dom";

import monkey from "../../assets/directory/monkey.png";
import { Listing, ListingSnippetImage } from "../apis/strims/network/v1/directory/directory";
import { Image, ImageType } from "../apis/strims/type/image";
import { DirectoryContext, DirectoryListing } from "../contexts/Directory";
import { useClient } from "../contexts/FrontendApi";
import useObjectURL from "../hooks/useObjectURL";
import jsonutil from "../lib/jsonutil";

interface DirectoryParams {
  networkKey: string;
}

const toEmbedService = (t: Listing.Embed.Service): string => {
  switch (t) {
    case Listing.Embed.Service.DIRECTORY_LISTING_EMBED_SERVICE_ANGELTHUMP:
      return "angelthump";
    case Listing.Embed.Service.DIRECTORY_LISTING_EMBED_SERVICE_TWITCH_STREAM:
      return "twitch-stream";
    case Listing.Embed.Service.DIRECTORY_LISTING_EMBED_SERVICE_TWITCH_VOD:
      return "twitch-vod";
    case Listing.Embed.Service.DIRECTORY_LISTING_EMBED_SERVICE_YOUTUBE:
      return "youtube";
  }
};

const formatUri = (networkKey: string, { content }: Listing): string => {
  switch (content.case) {
    case Listing.ContentCase.EMBED:
      return `/embed/${toEmbedService(content.embed.service)}/${content.embed.id}`;
    case Listing.ContentCase.MEDIA: {
      const mimeType = encodeURIComponent(content.media.mimeType);
      const swarmUri = encodeURIComponent(content.media.swarmUri);
      return `/player/${networkKey}?mimeType=${mimeType}&swarmUri=${swarmUri}`;
    }
    default:
      return "";
  }
};

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
    case ImageType.IMAGE_TYPE_SVG:
      return "image/svg+xml";
    case ImageType.IMAGE_TYPE_WEBP:
      return "image/webp";
  }
};

const useImage = (image: Image): string => useObjectURL(toFileType(image.type), image.data);

interface DirectoryGridImageProps extends ComponentProps<"img"> {
  fallback: string;
  source: ListingSnippetImage;
}

const DirectoryGridImage: React.FC<DirectoryGridImageProps> = ({
  fallback,
  source: { sourceOneof: source },
  ...imgProps
}) => {
  let url = "";
  switch (source.case) {
    case ListingSnippetImage.SourceOneofCase.URL:
      url = source.url;
      break;
    case ListingSnippetImage.SourceOneofCase.IMAGE:
      url = useImage(source.image);
      break;
  }

  return <img src={url || fallback} {...imgProps} />;
};

const formatNumberWithScale = (n: number, scale: number, unit: string): string =>
  `${(Math.round(n / (scale / 10)) / 10).toLocaleString()}${unit}`;

const formatNumber = (n: number) => {
  const scales: [string, number][] = [
    ["B", 1000000000],
    ["M", 1000000],
    ["K", 1000],
  ];
  for (const [unit, scale] of scales) {
    if (n >= scale) {
      return formatNumberWithScale(n, scale, unit);
    }
  }
  return n.toLocaleString();
};

interface DirectoryGridItemProps extends DirectoryListing {
  networkKey: string;
}

const DirectoryGridItem: React.FC<DirectoryGridItemProps> = ({
  listing,
  snippet,
  viewerCount,
  networkKey,
}) => {
  if (snippet === undefined) {
    return null;
  }

  // console.log(jsonutil.stringify(listing));
  // console.log(jsonutil.stringify(snippet));

  const title = snippet.title.trim();

  return (
    <div
      className={clsx({
        "directory_grid__item": true,
      })}
    >
      <Link className="directory_grid__item__link" to={formatUri(networkKey, listing)}>
        <DirectoryGridImage
          className="directory_grid__item__thumbnail"
          fallback={monkey}
          source={snippet.thumbnail}
        />
        <span className="directory_grid__item__viewer_count">
          {formatNumber(Number(snippet.viewerCount))}{" "}
          {snippet.viewerCount === BigInt(1) ? "viewer" : "viewers"}
        </span>
      </Link>
      <div className="directory_grid__item__channel">
        <DirectoryGridImage
          className="directory_grid__item__channel__logo"
          fallback={monkey}
          source={snippet.channelLogo}
        />
        <div className="directory_grid__item__channel__label">
          {title && (
            <span className="directory_grid__item__channel__title" title={title}>
              {title}
            </span>
          )}
          {snippet.channelName && (
            <span className="directory_grid__item__channel__name">{snippet.channelName}</span>
          )}
        </div>
      </div>
    </div>
  );
};

export interface DirectoryGridProps {
  listings: DirectoryListing[];
  networkKey: string;
}

export const DirectoryGrid: React.FC<DirectoryGridProps> = ({ listings, networkKey }) => (
  <div className="directory_grid">
    {listings.map((listing) => (
      <DirectoryGridItem key={listing.id.toString()} networkKey={networkKey} {...listing} />
    ))}
  </div>
);

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
        <DirectoryGrid listings={listings} networkKey={params.networkKey} />
      </div>
    </section>
  );
};

export default Directory;
