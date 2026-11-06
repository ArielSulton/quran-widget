"""pytest config for tools/: the ``slow`` marker is skipped unless selected with ``-m slow``."""
import pytest


def pytest_configure(config):
    config.addinivalue_line(
        "markers", "slow: runs a real Whisper model on a real recording (use -m slow)")


def pytest_collection_modifyitems(config, items):
    if "slow" in (config.getoption("-m") or ""):
        return
    skip = pytest.mark.skip(reason="slow test: run with -m slow")
    for item in items:
        if "slow" in item.keywords:
            item.add_marker(skip)
