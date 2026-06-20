"""Process-level bootstrap helpers for ``run_agent``.

Three concerns, all tied to ``AIAgent`` boot-time / runtime IO setup:

1. **Lazy OpenAI SDK import** — ``_load_openai_cls`` + ``_OpenAIProxy``
   defer the 240ms-ish ``from openai import OpenAI`` cost until first use,
   while preserving ``isinstance(client, OpenAI)`` checks and
   ``patch("run_agent.OpenAI", ...)`` test patterns.

2. **Crash-resistant stdio** — ``_SafeWriter`` wraps stdout/stderr so
   ``OSError: Input/output error`` from broken pipes (systemd, Docker,
   thread teardown races) cannot crash the agent.  ``_install_safe_stdio``
   applies the wrapper.

3. **HTTP proxy resolution** — ``_get_proxy_from_env`` reads
   ``HTTPS_PROXY`` / ``HTTP_PROXY`` / ``ALL_PROXY``;
   ``_get_proxy_for_base_url`` respects ``NO_PROXY`` for the given base URL.

``run_agent`` re-exports every name so existing
``from run_agent import _get_proxy_from_env`` imports keep working
unchanged.
"""

from __future__ import annotations

import os
import ipaddress
import sys
import urllib.request
from typing import Optional

from utils import base_url_hostname, normalize_proxy_url


IOLA_MANAGED_PROXY_URL = "https://iola-hermes.yasg.ru:9443"
IOLA_MANAGED_PROXY_DISABLED_VALUES = {"1", "true", "yes", "on"}
IOLA_MANAGED_PROXY_BYPASS_HOSTS = {
    "ai.api.cloud.yandex.net",
    "gigachat.devices.sberbank.ru",
    "ngw.devices.sberbank.ru",
}


# Cached at module level so we only pay the OpenAI SDK import cost once
# per process (after the first lazy load).
_OPENAI_CLS_CACHE = None


def _load_openai_cls() -> type:
    """Import and cache ``openai.OpenAI``."""
    global _OPENAI_CLS_CACHE
    if _OPENAI_CLS_CACHE is None:
        from openai import OpenAI as _cls
        _OPENAI_CLS_CACHE = _cls
    return _OPENAI_CLS_CACHE


class _OpenAIProxy:
    """Module-level proxy that looks like ``openai.OpenAI`` but imports lazily."""

    __slots__ = ()

    def __call__(self, *args, **kwargs):
        return _load_openai_cls()(*args, **kwargs)

    def __instancecheck__(self, obj):
        return isinstance(obj, _load_openai_cls())

    def __repr__(self):
        return "<lazy openai.OpenAI proxy>"


class _SafeWriter:
    """Transparent stdio wrapper that catches OSError/ValueError from broken pipes.

    When hermes-agent runs as a systemd service, Docker container, or headless
    daemon, the stdout/stderr pipe can become unavailable (idle timeout, buffer
    exhaustion, socket reset). Any print() call then raises
    ``OSError: [Errno 5] Input/output error``, which can crash agent setup or
    run_conversation() — especially via double-fault when an except handler
    also tries to print.

    Additionally, when subagents run in ThreadPoolExecutor threads, the shared
    stdout handle can close between thread teardown and cleanup, raising
    ``ValueError: I/O operation on closed file`` instead of OSError.

    This wrapper delegates all writes to the underlying stream and silently
    catches both OSError and ValueError. It is transparent when the wrapped
    stream is healthy.
    """

    __slots__ = ("_inner",)

    def __init__(self, inner):
        object.__setattr__(self, "_inner", inner)

    def write(self, data):
        try:
            return self._inner.write(data)
        except (OSError, ValueError):
            return len(data) if isinstance(data, str) else 0

    def flush(self):
        try:
            self._inner.flush()
        except (OSError, ValueError):
            pass

    def fileno(self):
        return self._inner.fileno()

    def isatty(self):
        try:
            return self._inner.isatty()
        except (OSError, ValueError):
            return False

    def __getattr__(self, name):
        return getattr(self._inner, name)


def _get_proxy_from_env() -> Optional[str]:
    """Read proxy URL from environment variables.

    Checks HTTPS_PROXY, HTTP_PROXY, ALL_PROXY (and lowercase variants) in order.
    Returns the first valid proxy URL found, or None if no proxy is configured.
    """
    for key in ("HTTPS_PROXY", "HTTP_PROXY", "ALL_PROXY",
                "https_proxy", "http_proxy", "all_proxy"):
        value = os.environ.get(key, "").strip()
        if value:
            return normalize_proxy_url(value)
    return None


def _is_iola_managed_proxy_disabled() -> bool:
    value = os.environ.get("IOLA_MANAGED_PROXY_DISABLED", "").strip().lower()
    return value in IOLA_MANAGED_PROXY_DISABLED_VALUES


def _host_matches(host: str, pattern: str) -> bool:
    host = host.strip().lower().rstrip(".")
    pattern = pattern.strip().lower().rstrip(".")
    return bool(host and pattern and (host == pattern or host.endswith(f".{pattern}")))


def _is_iola_managed_proxy_bypassed(host: str) -> bool:
    if not host:
        return False
    normalized = host.strip().lower().rstrip(".")
    if normalized in {"localhost", "127.0.0.1", "::1"}:
        return True
    try:
        address = ipaddress.ip_address(normalized.strip("[]"))
        if address.is_loopback or address.is_private or address.is_link_local:
            return True
    except ValueError:
        pass
    return any(_host_matches(host, pattern) for pattern in IOLA_MANAGED_PROXY_BYPASS_HOSTS)


def _get_proxy_for_base_url(base_url: Optional[str]) -> Optional[str]:
    """Return proxy for a base URL.

    User-configured HTTPS_PROXY/HTTP_PROXY/ALL_PROXY always wins. If the user
    did not configure a proxy, Hermes RU Iola uses its managed egress proxy for
    non-Russian remote providers while bypassing local endpoints, YandexGPT and
    GigaChat.
    """
    proxy = _get_proxy_from_env()
    if not proxy or not base_url:
        proxy = None

    host = base_url_hostname(base_url)
    if not host:
        return proxy

    try:
        if urllib.request.proxy_bypass_environment(host):
            return None
    except Exception:
        pass

    if proxy:
        return proxy

    if _is_iola_managed_proxy_disabled():
        return None
    if _is_iola_managed_proxy_bypassed(host):
        return None
    return normalize_proxy_url(IOLA_MANAGED_PROXY_URL)

def _install_safe_stdio() -> None:
    """Wrap stdout/stderr so best-effort console output cannot crash the agent."""
    for stream_name in ("stdout", "stderr"):
        stream = getattr(sys, stream_name, None)
        if stream is not None and not isinstance(stream, _SafeWriter):
            setattr(sys, stream_name, _SafeWriter(stream))


# Module-level proxy instance — drops in for ``openai.OpenAI``.  Imported as
# ``from agent.process_bootstrap import OpenAI`` (or re-exported via
# ``run_agent`` for legacy tests).
OpenAI = _OpenAIProxy()


__all__ = [
    "OpenAI",
    "_OpenAIProxy",
    "_load_openai_cls",
    "_SafeWriter",
    "_install_safe_stdio",
    "_get_proxy_from_env",
    "_get_proxy_for_base_url",
    "IOLA_MANAGED_PROXY_URL",
    "IOLA_MANAGED_PROXY_BYPASS_HOSTS",
]
