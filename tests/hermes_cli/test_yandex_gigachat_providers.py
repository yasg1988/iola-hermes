import pytest

from hermes_cli.auth import (
    DEFAULT_GIGACHAT_BASE_URL,
    DEFAULT_GIGACHAT_OAUTH_URL,
    DEFAULT_YANDEXGPT_BASE_URL,
    PROVIDER_REGISTRY,
    resolve_api_key_provider_credentials,
    resolve_provider,
)
from hermes_cli.config import OPTIONAL_ENV_VARS
from hermes_cli.models import CANONICAL_PROVIDERS, _PROVIDER_MODELS
from hermes_cli.provider_catalog import provider_catalog
from providers import get_provider_profile


@pytest.fixture(autouse=True)
def _clear_provider_env(monkeypatch):
    for key in (
        "YANDEX_API_KEY",
        "YANDEXGPT_API_KEY",
        "YANDEX_FOLDER_ID",
        "YANDEX_CLOUD_FOLDER_ID",
        "YC_FOLDER_ID",
        "YANDEXGPT_BASE_URL",
        "GIGACHAT_AUTH_KEY",
        "GIGACHAT_ACCESS_TOKEN",
        "GIGACHAT_BASE_URL",
        "GIGACHAT_OAUTH_URL",
        "GIGACHAT_SCOPE",
    ):
        monkeypatch.delenv(key, raising=False)
    monkeypatch.setattr("hermes_cli.config.load_env", lambda: {})
    monkeypatch.setattr("hermes_cli.auth._load_auth_store", lambda *args, **kwargs: {})


def test_yandex_and_gigachat_are_first_in_provider_picker():
    assert [entry.slug for entry in CANONICAL_PROVIDERS[:2]] == [
        "yandexgpt",
        "gigachat",
    ]
    assert [item.slug for item in provider_catalog()[:2]] == [
        "yandexgpt",
        "gigachat",
    ]


@pytest.mark.parametrize(
    "alias,expected",
    [
        ("yandex", "yandexgpt"),
        ("yandex-gpt", "yandexgpt"),
        ("giga", "gigachat"),
        ("sber", "gigachat"),
    ],
)
def test_provider_aliases(alias, expected):
    assert resolve_provider(alias) == expected


def test_yandex_registry_profile_and_env(monkeypatch):
    monkeypatch.setenv("YANDEX_API_KEY", "yandex-api-key")
    monkeypatch.setenv("YANDEX_FOLDER_ID", "folder-123")

    pconfig = PROVIDER_REGISTRY["yandexgpt"]
    assert pconfig.name == "YandexGPT"
    assert pconfig.api_key_env_vars == ("YANDEX_API_KEY", "YANDEXGPT_API_KEY")
    assert pconfig.base_url_env_var == "YANDEXGPT_BASE_URL"
    assert pconfig.inference_base_url == DEFAULT_YANDEXGPT_BASE_URL

    profile = get_provider_profile("yandex")
    assert profile is not None
    assert profile.get_default_headers()["x-folder-id"] == "folder-123"
    assert "gpt://{folder_id}/yandexgpt/latest" in _PROVIDER_MODELS["yandexgpt"]
    assert OPTIONAL_ENV_VARS["YANDEX_FOLDER_ID"]["password"] is False

    creds = resolve_api_key_provider_credentials("yandexgpt")
    assert creds["api_key"] == "yandex-api-key"
    assert creds["base_url"] == DEFAULT_YANDEXGPT_BASE_URL


def test_gigachat_registry_profile_and_env():
    pconfig = PROVIDER_REGISTRY["gigachat"]
    assert pconfig.name == "GigaChat"
    assert pconfig.api_key_env_vars == ("GIGACHAT_AUTH_KEY", "GIGACHAT_ACCESS_TOKEN")
    assert pconfig.base_url_env_var == "GIGACHAT_BASE_URL"
    assert pconfig.inference_base_url == DEFAULT_GIGACHAT_BASE_URL

    profile = get_provider_profile("giga")
    assert profile is not None
    assert profile.base_url == DEFAULT_GIGACHAT_BASE_URL
    assert "GigaChat" in _PROVIDER_MODELS["gigachat"]
    assert OPTIONAL_ENV_VARS["GIGACHAT_AUTH_KEY"]["password"] is True


def test_gigachat_preissued_access_token(monkeypatch):
    monkeypatch.setenv("GIGACHAT_ACCESS_TOKEN", "preissued-token")

    creds = resolve_api_key_provider_credentials("gigachat")

    assert creds["api_key"] == "preissued-token"
    assert creds["source"] == "GIGACHAT_ACCESS_TOKEN"
    assert creds["base_url"] == DEFAULT_GIGACHAT_BASE_URL


def test_gigachat_auth_key_is_exchanged_for_access_token(monkeypatch):
    import hermes_cli.auth as auth

    auth._GIGACHAT_TOKEN_CACHE.clear()
    calls = []

    class _Response:
        def raise_for_status(self):
            return None

        def json(self):
            return {"access_token": "runtime-token", "expires_at": 4_102_444_800_000}

    def fake_post(url, *, data, headers, timeout):
        calls.append((url, data, headers, timeout))
        return _Response()

    monkeypatch.setenv("GIGACHAT_AUTH_KEY", "auth-key")
    monkeypatch.setattr(auth.httpx, "post", fake_post)

    creds = resolve_api_key_provider_credentials("gigachat")
    cached = resolve_api_key_provider_credentials("gigachat")

    assert creds["api_key"] == "runtime-token"
    assert creds["source"] == "GIGACHAT_AUTH_KEY/oauth"
    assert cached["api_key"] == "runtime-token"
    assert len(calls) == 1
    url, data, headers, _timeout = calls[0]
    assert url == DEFAULT_GIGACHAT_OAUTH_URL
    assert data == {"scope": "GIGACHAT_API_PERS"}
    assert headers["Authorization"] == "Basic auth-key"
    assert headers["RqUID"]
