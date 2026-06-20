from providers import register_provider
from providers.base import ProviderProfile


provider = ProviderProfile(
    name="your-provider",
    aliases=("your-provider",),
    display_name="Ваш провайдер",
    description="OpenAI-compatible провайдер для Hermes RU Iola",
    signup_url="https://your-provider.example",
    env_vars=("YOUR_PROVIDER_API_KEY",),
    base_url="https://api.your-provider.example/v1",
    auth_type="api_key",
    default_aux_model="your-small-model",
    fallback_models=(
        "your-main-model",
        "your-small-model",
    ),
)

register_provider(provider)
