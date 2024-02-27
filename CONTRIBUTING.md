# Contributing

Contributions are welcome!

Please share any general questions, feature requests, or product feedback in our
[Convex Discord Community](https://convex.dev/community). We're particularly
excited to see what you build on Convex!

Please ensure that python code is formatted with
[black](https://pypi.org/project/black/) and markdown files are formatted with
[prettier](https://prettier.io/).

Run tests with

```
poetry install
poetry run pytest
```

Convex is a fast moving project developed by a dedicated team. We're excited to
contribute to the community by releasing this code, but we want to manage
expectations as well.

- We are a small company with a lot of product surface area.
- We value a cohesive developer experience for folks building applications
  across all of our languages and platforms.
- We value transparency in how we operate.

We're excited for community PRs. Be aware we may not get to it for a while.
Smaller PRs that only affect documentation/comments are easier to review and
integrate. For any larger or more fundamental changes, get in touch with us on
Discord before you put in too much work to see if it's consistent with our short
term plan. We think carefully about how our APIs contribute to a cohesive
product, so chatting up front goes a long way.

# Development notes

You'll need a Rust toolchain installed (e.g. through https://rustup.rs/) to
build this library when developing locally.

```sh
# install dependencies
poetry install --no-root

# build _convex
# This is requred to run tests and to use from other local installations (like smoke tests)
poetry run maturin dev

# run a test script
poetry run python simple_example.py

# interactive shell
poetry run python
>>> from convex import ConvexClient
>>> client = ConvexClint("https://flippant-cardinal-923.convex.cloud")
>>> print(client.query("users:list"))

# The wrapped Rust client can also be run in an interactive shell
>>> from _convex import PyConvexClient
>>> client = PyConvexClient("https://flippant-cardinal-923.convex.cloud")
>>> print(client.query("users:list"))

```

This package depends on the [convex](https://crates.io/crates/convex) package on
crates.io. Updating behavior defined in that package requires publishing an
update to that crate and updating the version required in this project.
