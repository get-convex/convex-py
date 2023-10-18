//! # Convex Client
//! The wrapped Python client for [Convex](https://convex.dev).
//!
//! Convex is the backend application platform with everything you need to build
//! your product. Convex clients can subscribe to queries and perform mutations
//! and actions. Check out the [Convex Documentation](https://docs.convex.dev) for more information.
//!
//! There is a Python layer above this package which re-exposes some of these
//! pyo3 structs in a more Pythonic way. Please refer to https://pypi.org/project/convex/
//! for official Python client documentation.

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

mod client;
pub use client::PyConvexClient;

mod query_result;
mod subscription;
