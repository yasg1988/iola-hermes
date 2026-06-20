"""YandexGPT provider profile."""

from __future__ import annotations

from providers import register_provider
from providers.base import ProviderProfile


class YandexGPTProfile(ProviderProfile):
    """Yandex AI Studio OpenAI-compatible endpoint."""

    def get_default_headers(self) -> dict[str, str]:
        headers = super().get_default_headers()
        try:
            from hermes_cli.config import get_env_value

            folder_id = (
                get_env_value("YANDEX_FOLDER_ID")
                or get_env_value("YANDEX_CLOUD_FOLDER_ID")
                or get_env_value("YC_FOLDER_ID")
                or ""
            ).strip()
        except Exception:
            folder_id = ""
        if folder_id:
            headers["x-folder-id"] = folder_id
        return headers


yandexgpt = YandexGPTProfile(
    name="yandexgpt",
    aliases=("yandex", "yandex-gpt", "yandex-ai-studio", "yandex-cloud"),
    display_name="YandexGPT",
    description="YandexGPT через Yandex Cloud AI Studio",
    signup_url="https://yandex.cloud/ru/docs/ai-studio/operations/get-api-key",
    env_vars=("YANDEX_API_KEY", "YANDEXGPT_API_KEY", "YANDEXGPT_BASE_URL"),
    base_url="https://ai.api.cloud.yandex.net/v1",
    auth_type="api_key",
    fallback_models=(
        "gpt://{folder_id}/yandexgpt/latest",
        "gpt://{folder_id}/yandexgpt-lite/latest",
        "gpt://{folder_id}/yandexgpt-pro/latest",
    ),
)

register_provider(yandexgpt)
