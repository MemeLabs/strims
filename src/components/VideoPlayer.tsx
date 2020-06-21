import * as React from "react";

import useSwarmMediaSource from "../hooks/useSwarmMediaSource";
import useVideo from "../hooks/useVideo";

interface Meme {
  reader?: any;
  mimeType?: any;
  useMediaSource?: any;
}

const SwarmPlayer = ({ reader, mimeType, useMediaSource = useSwarmMediaSource }: Meme) => {
  const [videoState, videoProps, videoControls] = useVideo();
  const [mediaSource, truncateMediaSource] = useMediaSource(reader, { mimeType });

  React.useEffect(() => {
    videoControls.setSrc(URL.createObjectURL(mediaSource));
    videoControls.play();
  }, [videoProps.ref, mediaSource]);

  React.useEffect(() => truncateMediaSource(60), [videoState.bufferEnd]);

  return (
    <video
      style={{ maxWidth: "100vw" }}
      onClick={(e) => e.preventDefault()}
      className="video_player__video"
      {...videoProps}
    />
  );
};

export default SwarmPlayer;
