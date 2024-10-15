import importlib

import convex
import convex.http_client


def test_version() -> None:
    # provisionally exists in >=3.8, no longer provisional in 3.10
    version: str = importlib.metadata.version("convex")  # pyright: ignore
    assert convex.__version__ == version

    http_client = convex.http_client.ConvexHttpClient("https://www.example.com")

    # Convex backend expects a dash in 0.1.6-a0, PyPI doesn't.
    if "a" in version:
        version = version.replace("a", "-a")
    assert http_client.headers["Convex-Client"] == f"python-{version}"
