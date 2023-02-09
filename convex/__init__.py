from typing import Any, Dict, Optional

import requests
from requests.exceptions import HTTPError

from .values import (
    CoercibleToConvexValue,
    ConvexMap,
    ConvexSet,
    ConvexValue,
    Id,
    JsonValue,
    convex_to_json,
    json_to_convex,
)

__all__ = [
    "Id",
    "JsonValue",
    "ConvexValue",
    "convex_to_json",
    "json_to_convex",
    "ConvexError",
    "ConvexClient",
    "ConvexSet",
    "ConvexMap",
]


__version__ = "0.1.1"


class ConvexExecutionError(Exception):
    """Convex execution error on server."""


class ConvexClient:
    def __init__(self, address: str) -> None:
        self.address: str = address
        self.auth: Optional[str] = None
        self.debug: bool = False
        self.headers = {"Convex-Client": f"python-convex-{__version__}"}

    def set_auth(self, value: str) -> None:
        self.auth = f"Bearer {value}"

    def set_admin_auth(self, admin_key: str) -> None:
        self.auth = f"Convex {admin_key}"

    def clear_auth(self) -> None:
        self.auth = None

    def set_debug(self, value: bool) -> None:
        "Sets whether the result log lines should be printed on the console."
        self.debug = value

    def _request(
        self, url: str, name: str, args: CoercibleToConvexValue
    ) -> ConvexValue:
        data: Dict[str, JsonValue] = {
            "path": name,
            "args": convex_to_json(args),
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

    def query(self, name: str, *args: CoercibleToConvexValue) -> Any:
        url = f"{self.address}/api/query"
        return self._request(url, name, args)

    def mutation(self, name: str, *args: CoercibleToConvexValue) -> Any:
        url = f"{self.address}/api/mutation"
        return self._request(url, name, args)

    def action(self, name: str, *args: CoercibleToConvexValue) -> Any:
        url = f"{self.address}/api/action"
        return self._request(url, name, args)
