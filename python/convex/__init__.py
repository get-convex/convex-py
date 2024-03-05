"""Python client for [Convex](https://convex.dev/) backed by a WebSocket."""

from __future__ import annotations

import logging
from typing import Any, Dict, Optional

from _convex import (
    PyConvexClient,
    PyQuerySetSubscription,
    PyQuerySubscription,
    init_logging,
)

from .values import (
    CoercibleToConvexValue,
    ConvexInt64,
    ConvexValue,
    JsonValue,
    coerce_args_to_convex,
    convex_to_json,
    json_to_convex,
)

# Configure Python's logging
logging.basicConfig(level=logging.DEBUG)

# Initialize Rust's logging system
init_logging()


__all__ = [
    "JsonValue",
    "ConvexValue",
    "convex_to_json",
    "json_to_convex",
    "ConvexError",
    "ConvexClient",
    "ConvexInt64",
]

__version__ = "0.6.0"  # Also update in pyproject.toml


class ConvexError(Exception):
    """Represents a ConvexError thrown on a Convex server.

    Unlike other errors thrown in Convex functions, ConvexErrors
    propagate to clients where they can be caught and inspected.
    """

    def __init__(self, message: str, data: Any):
        """Construct a ConvexError with given message (usually derived from data) and data.

        Unlike other errors thrown in Convex functions, ConvexErrors
        propagate to clients where they can be caught and inspected.
        """
        super().__init__(message)
        self.data = data


class ConvexExecutionError(Exception):
    """Convex execution error on server."""


FunctionArgs = Optional[Dict[str, CoercibleToConvexValue]]
SubscriberId = Any


class QuerySubscription:
    """This structure represents a single subscription to a query with args.

    It is returned by `ConvexClient::subscribe`. The subscription lives in the active
    query set for as long as this token stays in scope and is not unsubscribed.

    QuerySubscription provides a iterator of results to the query.
    """

    def __init__(
        self,
        inner: PyQuerySubscription,
        client: PyConvexClient,
        name: str,
        args: FunctionArgs = None,
    ) -> None:
        self.inner: PyQuerySubscription = inner
        self.client: PyConvexClient = client
        self.name: str = name
        self.args: FunctionArgs = args
        self.invalidated: bool = False

    # This addresses the "hitting ctrl-c while subscribed" issue
    # described in https://github.com/get-convex/convex/pull/18559.
    # If our program is interrupted/crashes while we're awaiting on a function
    # result from our backend, we might end up with a lost subscription.
    # This safeguard checks to make sure the inner subscription on the
    # Rust side exists. If not, we first re-subscribe.
    def safe_inner_sub(self) -> PyQuerySubscription:
        # Check if the subscription was unsubscribed.
        if self.invalidated:
            raise Exception("This subscription has been dropped")
        # In what situation would this occur? Why resubscribe?
        if not self.inner.exists():
            logging.warn("Retarting subscription dropped during possible ctrl-c")
            self.inner = self.client.subscribe(self.name, self.args)
        return self.inner

    @property
    def id(self) -> SubscriberId:
        """Returns an identifier for this subscription based on its query and args.

        This identifier can be used to find the result within a QuerySetSubscription
        as returned by `ConvexClient::watch_all()`
        """
        return self.safe_inner_sub().id

    def __iter__(self) -> QuerySubscription:
        return self

    def __next__(self) -> ConvexValue:
        result = self.safe_inner_sub().next()
        if result["type"] == "convexerror":
            raise ConvexError(result["message"], result["data"])
        return result["value"]

    def unsubscribe(self) -> None:
        """Unsubscribe from the query and drop this subscription from the active query set.

        Other subscriptions to the same query will be unaffected.
        """
        self.safe_inner_sub().unsubscribe()
        self.invalidated = True


class QuerySetSubscription:
    """A subscription to a consistent view of multiple queries.

    Provides an iterator, where each item contains a consistent view of the results
    of all the queries in the query set. Queries can be added to the query set via
    `ConvexClient::subscribe`.
    Queries can be removed from the query set via dropping the `QuerySubscription`
    token returned by `ConvexClient::subscribe`.

    Each item maps from `SubscriberId` to its latest result `ConvexValue`,
    or an error message/`ConvexError` if execution did not succeed.
    """

    def __init__(self, inner: PyQuerySetSubscription, client: PyConvexClient):
        self.inner: PyQuerySetSubscription = inner
        self.client: PyConvexClient = client

    def safe_inner_sub(self) -> PyQuerySetSubscription:
        if not self.inner.exists():
            self.inner = self.client.watch_all()
        return self.inner

    def __iter__(self) -> QuerySetSubscription:
        return self

    def __next__(self) -> Optional[Dict[SubscriberId, ConvexValue]]:
        result = self.safe_inner_sub().next()
        if not result:
            return result
        for k in result:
            result[k] = result[k]
        return result


class ConvexClient:
    """WebSocket-based Convex Client.

    A client to interact with a Convex deployment to perform
    queries/mutations/actions and manage query subscriptions.
    """

    # This client wraps PyConvexClient by
    # - implementing additional type convertions (e.g. tuples to arrays)
    # - making arguments dicts optional

    def __init__(self, deployment_url: str):
        """Construct a WebSocket-based client given the URL of a Convex deployment."""
        self.client: PyConvexClient = PyConvexClient(deployment_url)

    def subscribe(self, name: str, args: FunctionArgs = None) -> QuerySubscription:
        """Return a to subscription to the query `name` with optional `args`."""
        subscription = self.client.subscribe(name, args if args else {})
        return QuerySubscription(subscription, self.client, name, args if args else {})

    # Return Any because its more useful than the big union type ConvexValue.
    def query(self, name: str, args: FunctionArgs = None) -> Any:
        """Perform the query `name` with `args` returning the result."""
        result = self.client.query(name, coerce_args_to_convex(args))
        if result["type"] == "convexerror":
            raise ConvexError(result["message"], result["data"])
        return result["value"]

    def mutation(self, name: str, args: FunctionArgs = None) -> Any:
        """Perform the mutation `name` with `args` returning the result."""
        result = self.client.mutation(name, coerce_args_to_convex(args))
        if result["type"] == "convexerror":
            raise ConvexError(result["message"], result["data"])
        return result["value"]

    def action(self, name: str, args: FunctionArgs = None) -> Any:
        """Perform the action `name` with `args` returning the result."""
        result = self.client.action(name, coerce_args_to_convex(args))
        if result["type"] == "convexerror":
            raise ConvexError(result["message"], result["data"])
        return result["value"]

    def watch_all(self) -> QuerySetSubscription:
        """Return a QuerySetSubscription of all currently subscribed queries.

        This set changes over time as subscriptions are added and dropped.
        """
        set_subscription: PyQuerySetSubscription = self.client.watch_all()
        return QuerySetSubscription(set_subscription, self.client)

    def set_auth(self, token: str) -> None:
        """Set auth for use when calling Convex functions.

        Set it with a token that you get from your auth provider via their login
        flow. If `None` is passed as the token, then auth is unset (logging out).
        """
        self.client.set_auth(token)

    def clear_auth(self) -> None:
        """Clear any auth previously set."""
        self.client.set_auth(None)

    def set_admin_auth(self, admin_key: str) -> None:
        """Set admin auth for the deployment. Not typically required."""
        self.client.set_admin_auth(admin_key)
