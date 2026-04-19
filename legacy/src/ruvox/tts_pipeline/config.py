"""Pipeline configuration."""

from dataclasses import dataclass, field


@dataclass
class PipelineConfig:
    """Configuration for TTS text preprocessing pipeline."""

    # Code block handling mode: 'full' (read code) or 'brief' (description only)
    code_block_mode: str = "full"

    # URL detail level: 'full', 'domain_only', 'minimal'
    url_detail_level: str = "full"

    # Whether to read operators aloud
    read_operators: bool = True

    # IP address reading mode: 'numbers' or 'digits'
    ip_read_mode: str = "numbers"

    # Custom dictionaries (extend defaults)
    custom_it_terms: dict = field(default_factory=dict)
    custom_abbreviations: dict = field(default_factory=dict)

    # Debug mode
    debug: bool = False

    def update(self, **kwargs) -> None:
        """Update configuration values."""
        for key, value in kwargs.items():
            if hasattr(self, key):
                setattr(self, key, value)
