# Upcoming

# 0.2.0

- Both python `int` and `float` will coerce to a `number` type in Convex's JS
  environment. If you need a JS `bigint`, use a python `ConvexInt64`. Convex
  functions are written in JavaScript where `number` is used pervasively, so
  this change generally simplifies using Convex functions from python.
