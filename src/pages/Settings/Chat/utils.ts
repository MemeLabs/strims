// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";

import { EmoteFileType, EmoteScale, IEmote, IEmoteEffect } from "../../../apis/strims/chat/v1/chat";
import { ChatEmoteFormData } from "./ChatEmoteForm";

export const mimeTypeToFileType = (type: string): EmoteFileType => {
  switch (type) {
    case "image/png":
      return EmoteFileType.FILE_TYPE_PNG;
    case "image/gif":
      return EmoteFileType.FILE_TYPE_GIF;
    default:
      return EmoteFileType.FILE_TYPE_UNDEFINED;
  }
};

export const fileTypeToMimeType = (type: EmoteFileType): string => {
  switch (type) {
    case EmoteFileType.FILE_TYPE_PNG:
      return "image/png";
    case EmoteFileType.FILE_TYPE_GIF:
      return "image/gif";
    case EmoteFileType.FILE_TYPE_UNDEFINED:
      return "application/octet-stream";
  }
};

export const scaleToDOMScale = (type: EmoteScale): string => {
  switch (type) {
    case EmoteScale.EMOTE_SCALE_1X:
      return "1x";
    case EmoteScale.EMOTE_SCALE_2X:
      return "2x";
    case EmoteScale.EMOTE_SCALE_4X:
      return "4x";
  }
};

export const toEmoteProps = (data: ChatEmoteFormData): IEmote => {
  const effects: IEmoteEffect[] = [];
  if (data.animated) {
    effects.push({
      effect: {
        spriteAnimation: {
          frameCount: data.animationFrameCount,
          durationMs: data.animationDuration,
          iterationCount: data.animationIterationCount,
          endOnFrame: data.animationEndOnFrame,
          loopForever: data.animationLoopForever,
          alternateDirection: data.animationAlternateDirection,
        },
      },
    });
  }
  if (data.css) {
    effects.push({
      effect: {
        customCss: {
          // css: data.css,
        },
      },
    });
  }
  if (data.defaultModifiers?.length > 0) {
    effects.push({
      effect: {
        defaultModifiers: {
          modifiers: data.defaultModifiers.map(({ value }) => value),
        },
      },
    });
  }

  return {
    name: data.name,
    contributor: data.contributor && {
      name: data.contributor,
      link: data.contributorLink,
    },
    images: [
      {
        data: Base64.toUint8Array(data.image.data),
        fileType: mimeTypeToFileType(data.image.type),
        height: data.image.height,
        width: data.image.width,
        scale: data.scale.value,
      },
    ],
    effects,
    labels: data.labels.map(({ value }) => value),
    enable: data.enable,
  };
};
