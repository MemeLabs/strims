#!/usr/bin/env python3
import argparse
import shutil
import os


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "conf", type=str, default="/mnt/wg0.conf", help="location for new wg config"
    )
    args = parser.parse_args()

    wg_conf_file = "/etc/wireguard/wg0.conf"
    shutil.copy2(wg_conf_file, "/tmp/wg0.conf")
    shutil.copy2(args.conf, wg_conf_file)
    os.system("wg setconf wg0 <(wg-quick strip wg0)")

    return 0


if __name__ == "__main__":
    exit(main())
