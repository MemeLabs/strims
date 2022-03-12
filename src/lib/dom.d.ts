type VideoPresentationMode = "inline" | "picture-in-picture" | "fullscreen";

interface HTMLVideoElement {
  webkitSetPresentationMode: (mode: VideoPresentationMode) => void;
}
