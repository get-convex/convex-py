import importlib

import convex


def test_version() -> None:
    version = importlib.metadata.version("convex")
    assert convex.__version__ == version
    client = convex.ConvexClient("https://www.example.com")
    assert client.headers["Convex-Client"] == f"python-convex-{version}"
