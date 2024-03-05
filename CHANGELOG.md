# Upcoming

# 0.6.0

Version 0.6.0 is a rewrite which adds the ability to subscribe to live query
updates. The Python client now wraps the
[Convex Rust client](https://docs.rs/convex) using the PyO3[https://pyo3.rs/].

The API is a superset of the API in 0.5.1 but there are some behavior
differences. If you encounter unexpected incompatibilities consider the old
client, moved to `convex.http_client`.

One big change is that running a `client.query()` or mutation or action will
retry on network errors instead of throwing. If you were catching network errors
to implement retries in your code you should be able to get rid of this code.
The Convex Python client will retry indefinitely.

# 0.5.1

- Expose ConvexInt64 from the top level `convex` module.

# 0.5.0

- Remove ConvexMap and ConvexSet: these types are no longer allowed as arguments
  or returned by Convex functions.

  See the [NPM version 0.19.0](https://news.convex.dev/announcing-convex-0-19-0)
  release notes for more.

  If you need to communicate with a backend with functions that accept or return
  these types, _stay on version 0.4.0_.

- `ConvexClient.set_debug()` no longer applies to production deployments: logs
  from production deployments are no longer sent to clients, only appearing on
  the Convex dashboard.

- Add Support for `ConvexError`.

# 0.4.0

- Remove the `Id` class since document IDs are strings for Convex functions
  starting from
  [NPM version 0.17](https://news.convex.dev/announcing-convex-0-17-0/)
- Add warnings when calling functions on deprecated versions of Convex

# 0.3.0

This release corresponds with Convex npm release 0.13.0.

- `mutation()`, `action()`, and `query()` now take two arguments: the function
  name and an (optional) arguments dictionary. See
  https://news.convex.dev/announcing-convex-0-13-0/ for more about this change.

  If you need to communicate with a Convex backend with functions with
  positional arguments instead of a single arguments object, _stay on version
  0.2.0_.

# 0.2.0

- Both python `int` and `float` will coerce to a `number` type in Convex's JS
  environment. If you need a JS `bigint`, use a python `ConvexInt64`. Convex
  functions are written in JavaScript where `number` is used pervasively, so
  this change generally simplifies using Convex functions from python.
