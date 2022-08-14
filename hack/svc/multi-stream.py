import argparse
import re
import shutil
import time
from subprocess import Popen

parser = argparse.ArgumentParser()
parser.add_argument('-k', '--keys', nargs='+', type=str, dest='key_list',
                    help='<Required> Sets streaming keys', required=True)
args = parser.parse_args()


def ffmpeg_cmd(stream_link: str, stream_key: str) -> str:
    process = ["ffmpeg"]
    isUrl = re.match(
        "^(http[s]?:\/\/(www\.)?|ftp:\/\/(www\.)?|www\.){1}([0-9A-Za-z-\.@:%_\+~#=]+)+((\.[a-zA-Z]{2,3})+)(\/(.)*)?(\?(.)*)?", stream_link)
    if isUrl:
        process += ["-reconnect", "1", "-reconnect_at_eof",
                    "1", "-reconnect_delay_max", "10"]

    process += ["-re", "-y", "-i", stream_link,
                "-codec:a", "aac", "-c:v", "copy", "-f", "flv", "-flvflags", "no_duration_filesize",
                f"rtmp://0.0.0.0:1935/live/{stream_key}"]

    return " ".join(process)


def main() -> int:
    if shutil.which("ffmpeg") is None:
        raise Exception("ffmpeg is required")
    test_url = "http://commondatastorage.googleapis.com/gtv-videos-bucket/sample/ForBiggerBlazes.mp4"
    strims_to_open = []

    for stream_key in args.key_list:
        strims_to_open.append(ffmpeg_cmd(test_url, stream_key))

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
