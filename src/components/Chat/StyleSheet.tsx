import React, { useLayoutEffect, useRef, useState } from "react";

import { Emote, EmoteFileType, EmoteScale } from "../../apis/strims/chat/v1/chat";

const toMimeType = (fileType: EmoteFileType): string => {
  switch (fileType) {
    case EmoteFileType.FILE_TYPE_PNG:
      return "image/png";
    case EmoteFileType.FILE_TYPE_GIF:
      return "image/gif";
  }
};

const toCSSScale = (scale: EmoteScale): number => {
  switch (scale) {
    case EmoteScale.EMOTE_SCALE_1X:
      return 1;
    case EmoteScale.EMOTE_SCALE_2X:
      return 2;
    case EmoteScale.EMOTE_SCALE_4X:
      return 4;
  }
};

interface StyleSheetProps {
  liveEmotes: Emote[];
  styles: { [key: string]: string };
}

interface EmoteState {
  uris: string[];
  selector: string;
}

const StyleSheet: React.FC<StyleSheetProps> = ({ liveEmotes, styles }) => {
  const ref = useRef<HTMLStyleElement>(null);
  const [, setEmotes] = useState(new Map<Emote, EmoteState>());

  useLayoutEffect(() => {
    setEmotes((prev) => {
      const next = new Map(Array.from(prev.entries()));
      const added = liveEmotes.filter((e) => !prev.has(e));
      const removed = Array.from(prev.entries()).filter(([e]) => !liveEmotes.includes(e));

      removed.forEach(([e, { selector, uris }]) => {
        uris.forEach((uri) => URL.revokeObjectURL(uri));
        next.delete(e);

        for (let i = 0; i < ref.current.sheet.cssRules.length; ) {
          if (ref.current.sheet.cssRules.item(i).cssText.startsWith(selector)) {
            ref.current.sheet.deleteRule(i);
          }
        }
      });
      added.forEach((e) => {
        const uris = e.images.map(({ data, fileType }) =>
          URL.createObjectURL(new Blob([data], { type: toMimeType(fileType) }))
        );
        const selector = `.${styles[e.name]}`;
        const imageSet = e.images.map(({ scale }, i) => `url(${uris[i]}) ${toCSSScale(scale)}x`);
        const sample = e.images[0];
        const sampleScale = toCSSScale(sample.scale);
        const height = sample.height / sampleScale;
        const width = sample.width / sampleScale;

        ref.current.sheet.insertRule(
          [
            `${selector} {`,
            `background-image: image-set(${imageSet.join(", ")});`,
            `background-image: -webkit-image-set(${imageSet.join(", ")});`,
            `background-repeat: "no-repeat";`,
            `width: ${width}px;`,
            `height: ${height}px;`,
            `margin-top: calc(0.5em - ${height / 2}px);`,
            `margin-bottom: calc(0.5em - ${height / 2}px);`,
            `}`,
          ].join("")
        );

        next.set(e, { selector, uris });
      });

      return next;
    });
  }, [liveEmotes]);

  return <style ref={ref} />;
};

export default StyleSheet;
