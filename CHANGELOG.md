# Upcoming

# 0.4.0

- Remove the `Id` class since document IDs are strings for Convex functions
  starting from
  [NPM version 0.17](https://blog.convex.dev/announcing-convex-0-17-0/)
- Add warnings when calling functions on deprecated versions of Convex

# 0.3.0

This release corresponds with Convex npm release 0.13.0.

- `mutation()`, `action()`, and `query()` now take two arguments: the function
  name and an (optional) arguments dictionary. See
  https://blog.convex.dev/announcing-convex-0-13-0/ for more about this change.

  If you need to communicate with a Convex backend with functions with
  positional arguments instead of a single arguments object, _stay on version
  0.2.0_.

# 0.2.0

- Both python `int` and `float` will coerce to a `number` type in Convex's JS
  environment. If you need a JS `bigint`, use a python `ConvexInt64`. Convex
  functions are written in JavaScript where `number` is used pervasively, so
  this change generally simplifies using Convex functions from python.
