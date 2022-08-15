import argparse
import re
import os
import shutil
import sys
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument(
    "-k",
    "--keys",
    action="append",
    type=str,
    help="Set streaming keys",
    required=True,
)
parser.add_argument(
    "-a",
    "--address",
    type=str,
    help="Address of the RTMP server",
    default="0.0.0.0:1935",
)
parser.add_argument(
    "-s",
    "--source",
    type=str,
    help="Source to stream to the server",
    default="http://commondatastorage.googleapis.com/gtv-videos-bucket/sample/ForBiggerBlazes.mp4",
)


def ffmpeg_cmd(stream_link: str, stream_key: str, address: str) -> str:
    is_url = re.match(
        "^(http[s]?:\/\/(www\.)?|ftp:\/\/(www\.)?|www\.){1}([0-9A-Za-z-\.@:%_\+~#=]+)+((\.[a-zA-Z]{2,3})+)(\/(.)*)?(\?(.)*)?",
        stream_link,
    )

    process = ["ffmpeg"]

    if is_url:
        process += [
            "-reconnect",
            "1",
            "-reconnect_at_eof",
            "1",
            "-reconnect_delay_max",
            "10",
        ]

    process += [
        "-re",
        "-y",
        "-i",
        stream_link,
        "-codec:a",
        "aac",
        "-c:v",
        "copy",
        "-f",
        "flv",
        "-flvflags",
        "no_duration_filesize",
        f"rtmp://{address}/live/{stream_key}",
    ]

    return " ".join(process)


def main() -> int:
    args = parser.parse_args()

    if shutil.which("ffmpeg") is None:
        raise Exception("ffmpeg is required")

    processes = []
    for stream_key in args.keys:
        processes.append(
            subprocess.Popen(
                ffmpeg_cmd(args.source, stream_key, args.address),
                shell=True,
                # TODO: try to control output
                # stdout=subprocess.DEVNULL,
                # stderr=subprocess.STDOUT,
            )
        )

    for process in processes:
        try:
            process.wait()
        except KeyboardInterrupt:
            try:
                process.terminate()
            except OSError:
                pass
            process.wait()

    if sys.platform == "linux":
        os.system("stty sane")

    return 0


if __name__ == "__main__":
    exit(main())
