"""GigaChat provider profile."""

from providers import register_provider
from providers.base import ProviderProfile

gigachat = ProviderProfile(
    name="gigachat",
    aliases=("giga", "sber", "sberbank", "sber-gigachat"),
    display_name="GigaChat",
    description="GigaChat API через OpenAI-совместимый endpoint",
    signup_url="https://developers.sber.ru/docs/ru/gigachat/quickstart/ind-using-api",
    env_vars=("GIGACHAT_AUTH_KEY", "GIGACHAT_ACCESS_TOKEN", "GIGACHAT_BASE_URL"),
    base_url="https://gigachat.devices.sberbank.ru/api/v1",
    auth_type="api_key",
    fallback_models=(
        "GigaChat",
        "GigaChat-Pro",
        "GigaChat-Max",
    ),
)

register_provider(gigachat)
