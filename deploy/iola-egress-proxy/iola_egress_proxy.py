#!/usr/bin/env python3
"""Minimal TLS CONNECT proxy for Hermes RU Iola managed egress.

The service intentionally supports CONNECT only. It is not a general web proxy:
destination hosts are allowlisted, non-443 ports are denied by default, and no
request bodies are logged.
"""

from __future__ import annotations

import asyncio
import ipaddress
import logging
import os
import ssl
import time
from collections import defaultdict, deque
from dataclasses import dataclass


LOG_LEVEL = os.getenv("IOLA_EGRESS_LOG_LEVEL", "INFO").upper()
LISTEN_HOST = os.getenv("IOLA_EGRESS_LISTEN_HOST", "0.0.0.0")
LISTEN_PORT = int(os.getenv("IOLA_EGRESS_LISTEN_PORT", "9443"))
CERT_FILE = os.getenv("IOLA_EGRESS_CERT_FILE", "/etc/letsencrypt/live/iola-hermes.yasg.ru/fullchain.pem")
KEY_FILE = os.getenv("IOLA_EGRESS_KEY_FILE", "/etc/letsencrypt/live/iola-hermes.yasg.ru/privkey.pem")
CONNECT_TIMEOUT_SECONDS = float(os.getenv("IOLA_EGRESS_CONNECT_TIMEOUT", "15"))
IDLE_TIMEOUT_SECONDS = float(os.getenv("IOLA_EGRESS_IDLE_TIMEOUT", "300"))
RATE_LIMIT_WINDOW_SECONDS = float(os.getenv("IOLA_EGRESS_RATE_WINDOW", "60"))
RATE_LIMIT_MAX_CONNECTS = int(os.getenv("IOLA_EGRESS_RATE_MAX", "120"))
MAX_HEADER_BYTES = int(os.getenv("IOLA_EGRESS_MAX_HEADER_BYTES", "8192"))

ALLOWLIST = {
    "api.openai.com",
    "chatgpt.com",
    "api.anthropic.com",
    "openrouter.ai",
    "api.x.ai",
    "generativelanguage.googleapis.com",
    "api.deepseek.com",
    "api.moonshot.ai",
    "api.kimi.com",
    "api.githubcopilot.com",
    "api.groq.com",
    "api.mistral.ai",
    "api.together.xyz",
    "api.fireworks.ai",
    "api.perplexity.ai",
    "api.cohere.com",
    "router.huggingface.co",
    "integrate.api.nvidia.com",
    "api.minimax.io",
    "api.stepfun.ai",
    "api.novita.ai",
    "api.gmi-serving.com",
    "api.kilo.ai",
    "api.talos.com",
}

BYPASS_RU = {
    "ai.api.cloud.yandex.net",
    "gigachat.devices.sberbank.ru",
    "ngw.devices.sberbank.ru",
}

logging.basicConfig(
    level=getattr(logging, LOG_LEVEL, logging.INFO),
    format="%(asctime)s %(levelname)s %(message)s",
)
logger = logging.getLogger("iola-egress-proxy")


@dataclass(frozen=True)
class Target:
    host: str
    port: int


_rate_buckets: dict[str, deque[float]] = defaultdict(deque)


def _host_matches(host: str, pattern: str) -> bool:
    host = host.strip().lower().rstrip(".")
    pattern = pattern.strip().lower().rstrip(".")
    return bool(host and pattern and (host == pattern or host.endswith(f".{pattern}")))


def _is_private_host(host: str) -> bool:
    try:
        ip = ipaddress.ip_address(host)
    except ValueError:
        return False
    return ip.is_private or ip.is_loopback or ip.is_link_local or ip.is_multicast


def _is_allowed_host(host: str) -> bool:
    if _is_private_host(host):
        return False
    if any(_host_matches(host, pattern) for pattern in BYPASS_RU):
        return False
    return any(_host_matches(host, pattern) for pattern in ALLOWLIST)


def _rate_limited(peer_ip: str) -> bool:
    now = time.monotonic()
    bucket = _rate_buckets[peer_ip]
    while bucket and now - bucket[0] > RATE_LIMIT_WINDOW_SECONDS:
        bucket.popleft()
    if len(bucket) >= RATE_LIMIT_MAX_CONNECTS:
        return True
    bucket.append(now)
    return False


async def _read_header(reader: asyncio.StreamReader) -> bytes:
    data = bytearray()
    while b"\r\n\r\n" not in data:
        chunk = await asyncio.wait_for(reader.read(1024), timeout=10)
        if not chunk:
            break
        data.extend(chunk)
        if len(data) > MAX_HEADER_BYTES:
            raise ValueError("request header too large")
    return bytes(data)


def _parse_connect(header: bytes) -> Target:
    first_line = header.split(b"\r\n", 1)[0].decode("ascii", "replace")
    parts = first_line.split()
    if len(parts) != 3 or parts[0].upper() != "CONNECT":
        raise ValueError("only CONNECT is supported")
    authority = parts[1]
    if ":" not in authority:
        raise ValueError("CONNECT target must include port")
    host, port_text = authority.rsplit(":", 1)
    port = int(port_text)
    return Target(host=host.lower().strip("[]"), port=port)


async def _pipe(reader: asyncio.StreamReader, writer: asyncio.StreamWriter) -> None:
    try:
        while not reader.at_eof():
            data = await asyncio.wait_for(reader.read(65536), timeout=IDLE_TIMEOUT_SECONDS)
            if not data:
                break
            writer.write(data)
            await writer.drain()
    except (asyncio.TimeoutError, ConnectionError, OSError):
        pass
    finally:
        try:
            writer.close()
            await writer.wait_closed()
        except Exception:
            pass


async def _send_error(writer: asyncio.StreamWriter, status: str) -> None:
    writer.write(f"HTTP/1.1 {status}\r\nConnection: close\r\n\r\n".encode("ascii"))
    await writer.drain()


async def handle_client(reader: asyncio.StreamReader, writer: asyncio.StreamWriter) -> None:
    peer = writer.get_extra_info("peername")
    peer_ip = str(peer[0]) if peer else "unknown"
    target: Target | None = None
    try:
        if _rate_limited(peer_ip):
            await _send_error(writer, "429 Too Many Requests")
            return
        header = await _read_header(reader)
        target = _parse_connect(header)
        if target.port != 443:
            await _send_error(writer, "403 Forbidden")
            return
        if not _is_allowed_host(target.host):
            await _send_error(writer, "403 Forbidden")
            logger.warning("denied peer=%s target=%s:%s", peer_ip, target.host, target.port)
            return

        upstream_reader, upstream_writer = await asyncio.wait_for(
            asyncio.open_connection(target.host, target.port),
            timeout=CONNECT_TIMEOUT_SECONDS,
        )
        writer.write(b"HTTP/1.1 200 Connection Established\r\n\r\n")
        await writer.drain()
        logger.info("connect peer=%s target=%s:%s", peer_ip, target.host, target.port)
        await asyncio.gather(
            _pipe(reader, upstream_writer),
            _pipe(upstream_reader, writer),
        )
    except Exception as exc:
        logger.info("closed peer=%s target=%s error=%s", peer_ip, target, exc)
        try:
            await _send_error(writer, "502 Bad Gateway")
        except Exception:
            pass
    finally:
        try:
            writer.close()
            await writer.wait_closed()
        except Exception:
            pass


async def main() -> None:
    ssl_ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    ssl_ctx.load_cert_chain(CERT_FILE, KEY_FILE)
    server = await asyncio.start_server(
        handle_client,
        LISTEN_HOST,
        LISTEN_PORT,
        ssl=ssl_ctx,
        limit=MAX_HEADER_BYTES,
    )
    sockets = ", ".join(str(sock.getsockname()) for sock in server.sockets or [])
    logger.info("iola egress proxy listening on %s", sockets)
    async with server:
        await server.serve_forever()


if __name__ == "__main__":
    asyncio.run(main())
