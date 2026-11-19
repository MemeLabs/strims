# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""Presigned S3 POST policies (SigV4), and the upload that uses one: the
farm's instances put their flushes' files in S3 with no credentials of
their own.

A policy lets its holder POST any object under one key prefix of one
bucket, up to a size, until it expires: one signature for however many
files an instance writes, where a presigned PUT names one object. It's
signed here with the controller's credentials (`credentials`: the aws
CLI's, as `aws configure export-credentials` gives them), so it's good
only while those are: long-lived keys sign for as long as a policy asks,
a session's only until the session ends. The signer needs s3:PutObject
on the prefix.

`post` uploads a file under a policy as a multipart/form-data POST,
streamed from disk with only the standard library, so a worker needs
nothing installed.
"""

import base64
import datetime
import hashlib
import hmac
import http.client
import json
import os
import socket
import ssl
import subprocess
import time
import urllib.parse
import uuid
from pathlib import Path

# S3 takes a POST of up to 5 GB.
MAX_BYTES = 5 * 1024**3
CHUNK = 1 << 20
# A POST's TCP connect, over every address its host resolves to, and its
# TLS handshake take this long at most together, apart from its transfer's
# timeout, which can be all of a wind-down's or an interruption's window:
# one that stalls fails while the worker's retries (UPLOAD_RETRIES) still
# fit in it. Both take well under a second from the farthest region. The
# name's lookup before them is the system resolver's, bounded by its own
# timeouts (resolv.conf's: 5 s a try, 2 tries a server, by default).
CONNECT_S = 20


def credentials(profile: str | None = None) -> dict:
    """The aws CLI's credentials: {AccessKeyId, SecretAccessKey,
    SessionToken?, Expiration?}."""
    cmd = ["aws", "configure", "export-credentials", "--format", "process"]
    if profile:
        cmd += ["--profile", profile]
    p = subprocess.run(cmd, capture_output=True, text=True)
    if p.returncode:
        raise RuntimeError(f"aws configure export-credentials: {p.stderr.strip()}")
    return json.loads(p.stdout)


def _hmac(key: bytes, msg: str) -> bytes:
    return hmac.new(key, msg.encode(), hashlib.sha256).digest()


def policy(bucket: str, region: str, prefix: str, expires: datetime.datetime, creds: dict,
           max_bytes: int = MAX_BYTES, now: datetime.datetime | None = None) -> dict:
    """A POST policy for any key under `prefix` in `bucket` (in `region`)
    until `expires`, signed with `creds`: {"url", "prefix", "fields",
    "expires"}, the form fields every POST under it sends but `key`."""
    now = now or datetime.datetime.now(datetime.UTC)
    if creds.get("Expiration"):
        until = datetime.datetime.fromisoformat(creds["Expiration"].replace("Z", "+00:00"))
        if until < expires:
            raise RuntimeError(f"the signing credentials expire at {until:%Y-%m-%dT%H:%MZ}, before the "
                               f"policy's {expires:%Y-%m-%dT%H:%MZ}: sign with long-lived keys")
    date = now.strftime("%Y%m%d")
    amz_date = now.strftime("%Y%m%dT%H%M%SZ")
    credential = f"{creds['AccessKeyId']}/{date}/{region}/s3/aws4_request"
    fields = {
        "x-amz-algorithm": "AWS4-HMAC-SHA256",
        "x-amz-credential": credential,
        "x-amz-date": amz_date,
    }
    if creds.get("SessionToken"):
        fields["x-amz-security-token"] = creds["SessionToken"]
    doc = {
        "expiration": expires.astimezone(datetime.UTC).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "conditions": [
            {"bucket": bucket},
            ["starts-with", "$key", prefix],
            ["content-length-range", 0, max_bytes],
            *({k: v} for k, v in fields.items()),
        ],
    }
    encoded = base64.b64encode(json.dumps(doc).encode()).decode()
    key = _hmac(_hmac(_hmac(_hmac(f"AWS4{creds['SecretAccessKey']}".encode(), date), region), "s3"), "aws4_request")
    fields["policy"] = encoded
    fields["x-amz-signature"] = hmac.new(key, encoded.encode(), hashlib.sha256).hexdigest()
    return {
        "url": f"https://{bucket}.s3.{region}.amazonaws.com/",
        "prefix": prefix,
        "fields": fields,
        "expires": doc["expiration"],
    }


def body_size(pol: dict, key: str, path: Path) -> int:
    """The bytes a POST of `path` as `key` sends: the form and the file."""
    return len(form_head(pol, key, path.name, "x" * 32)) + os.path.getsize(path) + len(form_tail("x" * 32))


def form_head(pol: dict, key: str, name: str, boundary: str) -> bytes:
    head = b""
    for field, value in [("key", key), *pol["fields"].items()]:
        head += (f"--{boundary}\r\nContent-Disposition: form-data; name=\"{field}\"\r\n\r\n{value}\r\n").encode()
    return head + (f"--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{name}\"\r\n"
                   "Content-Type: application/octet-stream\r\n\r\n").encode()


def form_tail(boundary: str) -> bytes:
    return f"\r\n--{boundary}--\r\n".encode()


def connect(scheme: str, netloc: str, within: float, timeout: float) -> socket.socket:
    """A socket to `netloc` (host[:port]), connected, and for https its TLS
    handshake done, within `within` seconds in all, over each address its
    name resolves to in turn; then `timeout` for each read and write."""
    deadline = time.monotonic() + within
    parts = urllib.parse.urlsplit(f"{scheme}://{netloc}")
    name, port = parts.hostname, parts.port or (443 if scheme == "https" else 80)

    def left() -> float:
        rest = deadline - time.monotonic()
        if rest <= 0:
            raise TimeoutError(f"{netloc}: not connected within {within:g} s")
        return rest

    sock, err = None, None
    for family, kind, proto, _, addr in socket.getaddrinfo(name, port, type=socket.SOCK_STREAM):
        s = socket.socket(family, kind, proto)
        try:
            s.settimeout(left())
            s.connect(addr)
        except TimeoutError:
            s.close()
            raise  # the deadline's
        except OSError as e:
            s.close()
            err = e
            continue
        sock = s
        break
    if sock is None:
        raise err or OSError(f"{netloc}: no address")
    try:
        if scheme == "https":
            sock.settimeout(left())
            sock = ssl.create_default_context().wrap_socket(sock, server_hostname=name)
        sock.settimeout(timeout)
    except BaseException:
        sock.close()
        raise
    return sock


def post(pol: dict, key: str, path: Path, timeout: float = 120) -> None:
    """Uploads the file `path` as `key` (under the policy's prefix) by the
    policy `pol`, streamed: connected within CONNECT_S (connect), then each
    read and write of the transfer within `timeout`. Raises RuntimeError on
    a refusal, TimeoutError on a timeout."""
    if not key.startswith(pol["prefix"]):
        raise ValueError(f"{key} is outside the policy's prefix {pol['prefix']}")
    boundary = uuid.uuid4().hex
    head, tail = form_head(pol, key, path.name, boundary), form_tail(boundary)
    size = os.path.getsize(path)
    scheme, _, rest = pol["url"].partition("://")
    host, _, base = rest.partition("/")
    # The request over the socket connect made, TLS's or not.
    conn = http.client.HTTPConnection(host, timeout=timeout)
    try:
        conn.sock = connect(scheme, host, min(timeout, CONNECT_S), timeout)
        conn.putrequest("POST", "/" + base)
        conn.putheader("Content-Type", f"multipart/form-data; boundary={boundary}")
        conn.putheader("Content-Length", str(len(head) + size + len(tail)))
        conn.endheaders()
        conn.send(head)
        with open(path, "rb") as f:
            for chunk in iter(lambda: f.read(CHUNK), b""):
                conn.send(chunk)
        conn.send(tail)
        r = conn.getresponse()
        body = r.read()
        if r.status not in (200, 201, 204):
            raise RuntimeError(f"POST {key}: {r.status} {body[:500].decode(errors='replace')}")
    finally:
        conn.close()
