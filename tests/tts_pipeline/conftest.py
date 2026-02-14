"""Pytest configuration and fixtures."""

import pytest
from ruvox.tts_pipeline import (
    PipelineConfig,
    TTSPipeline,
    EnglishNormalizer,
    AbbreviationNormalizer,
    NumberNormalizer,
    URLPathNormalizer,
    SymbolNormalizer,
    CodeIdentifierNormalizer,
    CodeBlockHandler,
)


@pytest.fixture
def config():
    """Default pipeline configuration."""
    return PipelineConfig()


@pytest.fixture
def pipeline(config):
    """Default pipeline instance."""
    return TTSPipeline(config)


@pytest.fixture
def english_normalizer():
    """English words normalizer."""
    return EnglishNormalizer()


@pytest.fixture
def abbrev_normalizer():
    """Abbreviations normalizer."""
    return AbbreviationNormalizer()


@pytest.fixture
def number_normalizer():
    """Numbers normalizer."""
    return NumberNormalizer()


@pytest.fixture
def url_normalizer():
    """URL/path normalizer."""
    return URLPathNormalizer()


@pytest.fixture
def symbol_normalizer():
    """Symbol normalizer."""
    return SymbolNormalizer()


@pytest.fixture
def code_normalizer(english_normalizer):
    """Code identifier normalizer."""
    return CodeIdentifierNormalizer()


@pytest.fixture
def code_block_handler():
    """Code block handler."""
    return CodeBlockHandler()
