"""The official Python client for [Convex](https://convex.dev/)."""
import warnings
from typing import Any, Dict, Optional

import requests
from requests.exceptions import HTTPError

from .values import (
    CoercibleToConvexValue,
    ConvexMap,
    ConvexSet,
    ConvexValue,
    JsonValue,
    convex_to_json,
    json_to_convex,
)

__all__ = [
    "JsonValue",
    "ConvexValue",
    "convex_to_json",
    "json_to_convex",
    "ConvexError",
    "ConvexClient",
    "ConvexSet",
    "ConvexMap",
]


__version__ = "0.4.0"  # Also update in pyproject.toml


class ConvexExecutionError(Exception):
    """Convex execution error on server."""


FunctionArgs = Optional[Dict[str, CoercibleToConvexValue]]


class ConvexClient:
    """Client for communicating with convex."""

    def __init__(self, address: str) -> None:
        """Instantiate a `ConvexClient` that speaks to a deployment at `address`."""
        self.address: str = address
        self.auth: Optional[str] = None
        self.debug: bool = False
        self.headers = {"Convex-Client": f"python-convex-{__version__}"}

    def set_auth(self, value: str) -> None:
        """Set auth for use when calling Convex functions."""
        self.auth = f"Bearer {value}"

    def set_admin_auth(self, admin_key: str) -> None:
        """Set admin auth for the deployment. Not typically required."""
        self.auth = f"Convex {admin_key}"

    def clear_auth(self) -> None:
        """Clear any auth previously set."""
        self.auth = None

    def set_debug(self, value: bool) -> None:
        """Set whether the result log lines should be printed on the console."""
        self.debug = value

    def _request(self, url: str, name: str, args: FunctionArgs) -> ConvexValue:
        if args is None:
            args = {}
        if not type(args) is dict:
            raise Exception(
                f"Arguments to a Convex function must be a dictionary. Received: {args}"
            )

        data: Dict[str, JsonValue] = {
            "path": name,
            "args": [convex_to_json(args)],
        }
        if self.debug:
            data["debug"] = self.debug

        headers = self.headers.copy()
        if self.auth is not None:
            headers["Authorization"] = self.auth

        r = requests.post(url, json=data, headers=headers)
        response = r.json()

        if "logLines" in response:
            for line in response["logLines"]:
                print(line)

        deprecation_state = r.headers.get("x-convex-deprecation-state")
        deprecation_msg = r.headers.get("x-convex-deprecation-message")
        if deprecation_state and deprecation_msg:
            warnings.warn(f"{deprecation_state}: {deprecation_msg}", stacklevel=1)

        try:
            r.raise_for_status()
        except HTTPError:
            raise Exception(
                f"{r.status_code} {response['code']}: {response['message']}"
            )

        if response["status"] == "success":
            return json_to_convex(response["value"])
        if response["status"] == "error":
            raise ConvexExecutionError(response["errorMessage"])
        raise Exception("Received unexpected response from Convex server.")

    def query(self, name: str, args: FunctionArgs = None) -> Any:
        """Run a query on Convex."""
        url = f"{self.address}/api/query"
        return self._request(url, name, args)

    def mutation(self, name: str, args: FunctionArgs = None) -> Any:
        """Run a mutation on Convex."""
        url = f"{self.address}/api/mutation"
        return self._request(url, name, args)

    def action(self, name: str, args: FunctionArgs = None) -> Any:
        """Run an action on Convex."""
        url = f"{self.address}/api/action"
        return self._request(url, name, args)
