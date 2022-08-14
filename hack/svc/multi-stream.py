import json
import os
import shutil
import time
import re
import getopt
import sys
from subprocess import Popen


class strims_ffmpeg:
    def __init__(self, hostAddress="0.0.0.0") -> None:
        self.host_address = hostAddress

    def ffmpeg_cmd(self, streamLink: str, streamKey: str) -> str:
        process = ["ffmpeg"]
        isUrl = re.match(
            "^(http[s]?:\/\/(www\.)?|ftp:\/\/(www\.)?|www\.){1}([0-9A-Za-z-\.@:%_\+~#=]+)+((\.[a-zA-Z]{2,3})+)(\/(.)*)?(\?(.)*)?", streamLink)
        if isUrl:
            process += ["-reconnect", "1", "-reconnect_at_eof",
                        "1", "-reconnect_delay_max", "10"]

        process += ["-re", "-y", "-i", streamLink,
                    "-codec:a", "aac", "-c:v", "copy", "-f", "flv", "-flvflags", "no_duration_filesize",
                    f"rtmp://{self.hostAddress}:1935/live/{streamKey}"]

        return " ".join(process)

    def main(self) -> int:
        if shutil.which("ffmpeg") is None and shutil.which("ffprobe") is None:
            raise Exception("ffmpeg and ffprobe is required")
        strims = {"default": {"key": "", "link": ""},
                  "streams": ["https://devstreaming-cdn.apple.com/videos/streaming/examples/img_bipbop_adv_example_fmp4/master.m3u8",
                              "https://devstreaming-cdn.apple.com/videos/streaming/examples/img_bipbop_adv_example_fmp4/master.m3u8"],  "keys": sys.argv[1:]}
        strims_to_open = []

        if len(strims["streams"]) == 0:
            try:
                strimsToOpen.append(self.startStream(
                    strims["default"]["link"], strims["default"])["key"])
            except TypeError:
                print(
                    "\033[91m[+] Configuration file is missing data... \033[0m")
        else:
            for stream in strims["streams"]:
                strimsToOpen.append(self.startStream(
                    strims["streams"][s], strims["keys"][s]))

        procs = [Popen(i) for i in strimsToOpen]
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
    exit(strims_ffmpeg().main())
