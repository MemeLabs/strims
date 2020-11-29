#!/usr/bin/env python3
import argparse
import shutil
import subprocess


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("conf", type=str, default="/mnt/wg0.conf")
    args = parser.parse_args()

    # TODO(jbpratt): We can improve this with the following:
    # 1. temp file desc
    # 2. run subproccess with quick strip
    # 3. run wg setconf
    # 4. communicate

    wg_conf_file = "/etc/wireguard/wg0.conf"
    shutil.copy2(wg_conf_file, "/tmp/wg0.conf")
    shutil.copy2(args.conf, wg_conf_file)
    # subprocess.run(["bash", "-c", "wg", "setconf", "wg0", "<(wg-quick strip wg0)"])
    subprocess.run(["bash", "-c", "wg-quick down wg0"])
    subprocess.run(["bash", "-c", "wg-quick up wg0"])

    return 0


if __name__ == "__main__":
    exit(main())
