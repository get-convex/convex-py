# Convex

The official Python client for [Convex](https://convex.dev/).

![PyPI](https://img.shields.io/pypi/v/convex?label=convex&logo=pypi)
![GitHub](https://img.shields.io/github/license/get-convex/convex-py)

Write and read data from a Convex backend with queries, mutations, and actions.
Get up and running at [docs.convex.dev](https://docs.convex.dev/home).

Installation:

    pip install convex

Basic usage:

```python
>>> from convex import ConvexClient
>>> client = ConvexClient('https://example-lion-123.convex.cloud')
>>> messages = client.query("messages:list")
>>> from pprint import pprint
>>> pprint(messages)
[{'_creationTime': 1668107495676.2854,
  '_id': '2sh2c7pn6nyvkexbdsfj66vd9h5q3hg',
  'author': 'Tom',
  'body': 'Have you tried Convex?'},
 {'_creationTime': 1668107497732.2295,
  '_id': '1f053fgh2tt2fc93mw3sn2x09h5bj08',
  'author': 'Sarah',
  'body': "Yeah, it's working pretty well for me."}]
>>> client.mutation("messages:send", dict(author="Me", body="Hello!"))
>>> for messages in client.subscribe("messages:list", {}):
...     print(len(messages))
...
3
<this for loop lasts until you break out with ctrl-c>
```

To find the url of your convex backend, open the deployment you want to work
with in the appropriate project in the
[Convex dashboard](https://dashboard.convex.dev) and click "Settings" where the
Deployment URL should be visible. To find out which queries, mutations, and
actions are available check the Functions pane in the dashboard.

To see logs emitted from Convex functions, set the debug mode to True.

```python
>>> client.set_debug(True)
```

To provide authentication for function execution, call `set_auth()`.

```python
>>> client.set_auth("token-from-authetication-flow")
```

To authenticate as an admin, allowing you to run internal functions not exposed
to the public internet, call `set_admin_auth()`.

```python
>>> client.set_admin_auth("admin-key")
```

[Join us on Discord](https://www.convex.dev/community) to get your questions
answered or share what you're doing with Convex. If you're just getting started,
see https://docs.convex.dev to see how to quickly spin up a backend that does
everything you need in the Convex cloud.

# Convex types

Convex backend functions are written in JavaScript, so arguments passed to
Convex RPC functions in Python are serialized, sent over the network, and
deserialized into JavaScript objects. To learn about Convex's supported types
see https://docs.convex.dev/using/types.

In order to call a function that expects a JavaScript type, use the
corresponding Python type or any other type that coerces to it. Values returned
from Convex will be of the corresponding Python type.

| JavaScript Type                                                                                             | Python Type                                                                                                                    | Example              | Other Python Types that Convert |
| ----------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ | -------------------- | ------------------------------- |
| [null](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Data_structures#null_type)                   | [None](https://docs.python.org/3/library/stdtypes.html#the-null-object)                                                        | `None`               |                                 |
| [bigint](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Data_structures#bigint_type)               | ConvexInt64 (see below)                                                                                                        | `ConvexInt64(2**60)` |                                 |
| [number](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Data_structures#number_type)               | [float](https://docs.python.org/3/library/functions.html#float) or [int](https://docs.python.org/3/library/functions.html#int) | `3.1`, `10`          |                                 |
| [boolean](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Data_structures#boolean_type)             | [bool](https://docs.python.org/3/library/functions.html#bool)                                                                  | `True`, `False`      |                                 |
| [string](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Data_structures#string_type)               | [str](https://docs.python.org/3/library/stdtypes.html#str)                                                                     | `'abc'`              |                                 |
| [ArrayBuffer](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/ArrayBuffer) | [bytes](https://docs.python.org/3/library/stdtypes.html#bytes)                                                                 | `b'abc'`             | ArrayBuffer                     |
| [Array](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Array)             | [list](https://docs.python.org/3/library/stdtypes.html#list)                                                                   | `[1, 3.2, "abc"]`    | tuple, collections.abc.Sequence |
| [object](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Data_structures#objects)                   | [dict](https://docs.python.org/3/library/stdtypes.html#dict)                                                                   | `{a: "abc"}`         | collections.abc.Mapping         |

### Ints and Floats

While
[Convex supports storing Int64s and Float64s](https://docs.convex.dev/using/types#convex-types),
idiomatic JavaScript pervasively uses the (floating point) `Number` type. In
Python `float`s are often understood to contain the `int`s: the `float` type
annotation is
[generally understood as `Union[int, float]`](https://peps.python.org/pep-0484/#the-numeric-tower).

Therefore, the Python Convex client converts Python's `float`s and `int`s to a
`Float64` in Convex.

To specify a JavaScript BigInt, use the ConvexInt64 class. Functions which
return JavaScript BigInts will return ConvexInt64 instances.

# Convex Errors

The Python client supports the `ConvexError` type to hold application errors
that are propagated from your Convex functions. To learn about how to throw
`ConvexError`s see
https://docs.convex.dev/functions/error-handling/application-errors.

On the Python client, `ConvexError`s are Exceptions with a `data` field that
contains some `ConvexValue`. Handling application errors from the Python client
might look something like this:

```python
import convex
client = convex.ConvexClient('https://happy-animal-123.convex.cloud')

try:
    client.mutation("messages:sendMessage", {body: "hi", author: "anjan"})
except convex.ConvexError as err:
    if isinstance(err.data, dict):
        if "code" in err.data and err.data["code"] == 1:
            # do something
        else:
            # do something else
    elif isinstance(err.data, str):
        print(err.data)
except Exception as err:
    # log internally
```

# Pagination

[Paginated queries](https://docs.convex.dev/database/pagination) are queries
that accept pagination options as an argument and can be called repeatedly to
produce additional "pages" of results.

For a paginated query like this:

```javascript
import { query } from "./_generated/server";

export default query({
  handler: async ({ db }, { paginationOpts }) => {
    return await db.query("messages").order("desc").paginate(paginationOpts);
  },
});
```

and returning all results 5 at a time in Python looks like this:

```python
import convex
client = convex.ConvexClient('https://happy-animal-123.convex.cloud')

done = False
cursor = None
data = []

while not done:
    result = client.query('listMessages', {"paginationOpts": {"numItems": 5, "cursor": cursor}})
    cursor = result['continueCursor']
    done = result["isDone"]
    data.extend(result['page'])
    print('got', len(result['page']), 'results')

print('collected', len(data), 'results')
```

# Versioning

While we are pre-1.0.0, we'll update the minor version for large changes, and
the patch version for small bugfixes. We may make backwards incompatible changes
to the python client's API, but we will limit those to minor version bumps.
