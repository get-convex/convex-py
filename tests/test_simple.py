from _convex import PyConvexClient
from convex import ConvexClient, __version__


def test_rust_instantiation() -> None:
    PyConvexClient("https://made-up-animal.convex.cloud", __version__)


def test_instantiation() -> None:
    # this is currently completely different code (HTTP client)
    ConvexClient("https://made-up-animal.convex.cloud")
