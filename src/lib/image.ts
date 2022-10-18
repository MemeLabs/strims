// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";
import { invert } from "lodash";

import { Image, ImageType } from "../apis/strims/type/image";
import { ImageValue } from "../components/Form";

const imageTypeToFileType: { [imageType: number]: string } = {
  [ImageType.IMAGE_TYPE_APNG]: "image/apng",
  [ImageType.IMAGE_TYPE_AVIF]: "image/avif",
  [ImageType.IMAGE_TYPE_GIF]: "image/gif",
  [ImageType.IMAGE_TYPE_JPEG]: "image/jpeg",
  [ImageType.IMAGE_TYPE_PNG]: "image/png",
  [ImageType.IMAGE_TYPE_WEBP]: "image/webp",
  [ImageType.IMAGE_TYPE_SVG]: "image/svg+xml",
};
const fileTypeToImageType = invert(imageTypeToFileType);

export const toFileType = (t: ImageType) => imageTypeToFileType[t];

export const toImageType = (t: string): ImageType => Number(fileTypeToImageType[t]);

export const fromFormImageValue = ({ data, type, ...v }: ImageValue) =>
  new Image({
    data: Base64.toUint8Array(data),
    type: toImageType(type),
    ...v,
  });

export const toFormImageValue = ({ data, type, ...v }: Image): ImageValue => ({
  data: Base64.fromUint8Array(data),
  type: toFileType(type),
  ...v,
});

export const createImageObjectURL = ({ data, type }: Image) =>
  URL.createObjectURL(new Blob([data], { type: toFileType(type) }));
