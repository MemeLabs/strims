import re
import shutil
import sys
import time
from subprocess import Popen


def ffmpeg_cmd(streamLink: str, streamKey: str) -> str:
    process = ["ffmpeg"]
    isUrl = re.match(
        "^(http[s]?:\/\/(www\.)?|ftp:\/\/(www\.)?|www\.){1}([0-9A-Za-z-\.@:%_\+~#=]+)+((\.[a-zA-Z]{2,3})+)(\/(.)*)?(\?(.)*)?", streamLink)
    if isUrl:
        process += ["-reconnect", "1", "-reconnect_at_eof",
                    "1", "-reconnect_delay_max", "10"]

    process += ["-re", "-y", "-i", streamLink,
                "-codec:a", "aac", "-c:v", "copy", "-f", "flv", "-flvflags", "no_duration_filesize",
                f"rtmp://0.0.0.0:1935/live/{streamKey}"]

    return " ".join(process)


def main() -> int:
    if shutil.which("ffmpeg") is None and shutil.which("ffprobe") is None:
        raise Exception("ffmpeg and ffprobe is required")
    strims = {"streams": ["http://commondatastorage.googleapis.com/gtv-videos-bucket/sample/ForBiggerBlazes.mp4",
                          "http://commondatastorage.googleapis.com/gtv-videos-bucket/sample/ForBiggerBlazes.mp4"],  "keys": sys.argv[1:]}
    strims_to_open = []

    for s in range(0, len(strims["streams"])):
        strims_to_open.append(ffmpeg_cmd(
            strims["streams"][s], strims["keys"][s]))

    procs = [Popen(i) for i in strims_to_open]
    for p in procs:
        try:
            p.wait()
        except KeyboardInterrupt:
            try:
                p.terminate()
            except OSError:
                pass
            p.wait()
            time.sleep(2)
    return 0


if __name__ == "__main__":
    exit(main())
