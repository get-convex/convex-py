# Convex

The official Python client for [Convex](https://convex.dev/).

![PyPI](https://img.shields.io/pypi/v/convex?label=convex&logo=pypi)
![GitHub](https://img.shields.io/github/license/get-convex/convex-py)

Convex is the TypeScript-native programmable database for the web. No need for
backend containers, caches, queues, and query languages. Convex replaces all of
them with a few simple APIs.

This Python client can write and read data from a Convex backend with queries,
mutations, and actions. Get up and running at
[docs.convex.dev](https://docs.convex.dev/introduction/).

Installation:

    pip install convex

Basic usage:

```python
>>> from convex import ConvexClient
>>> client = ConvexClient('https://example-lion-123.convex.cloud')
>>> messages = client.query("listMessages")
>>> from pprint import pprint
>>> pprint(messages)
[{'_creationTime': 1668107495676.2854,
  '_id': Id(table_name='messages', id='c09S884lW4kTLdQMtu2ravf'),
  'author': 'Tom',
  'body': 'Have you tried Convex?'},
 {'_creationTime': 1668107497732.2295,
  '_id': Id(table_name='messages', id='G3m0cCQp65GQDfUjUDnTPEj'),
  'author': 'Sarah',
  'body': "Yeah, it's working pretty well for me."}]
>>> client.mutation("sendMessage")
```

To find the url of your convex backend, open the deployment you want to work
with in the appropriate project in the
[Convex dashboard](https://dashboard.convex.dev) and click "Settings" where the
Deployment URL should be visible. To find out which queries, mutations, and
actions are available check the Functions pane in the Dashboard

To see logs emitted from Convex functions, set the debug mode to True.

```python
>>> client.set_debug(True)
```

To provide authentication for function execution, call `set_auth()`.

```python
>>> client.set_auth("token-from-authetication-flow")
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

| JavaScript Type                                                                                             | Python Type                                                                                                                    | Example                           | Other Python Types that Convert     |
| ----------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ | --------------------------------- | ----------------------------------- |
| [Id](https://docs.convex.dev/api/classes/values.GenericId)                                                  | Id (see below)                                                                                                                 | `Id(tableName, id)`               |                                     |
| [null](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Data_structures#null_type)                   | [None](https://docs.python.org/3/library/stdtypes.html#the-null-object)                                                        | `None`                            |                                     |
| [bigint](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Data_structures#bigint_type)               | ConvexBigInt (see below)                                                                                                       | `ConvexInt64(2**60)`              |                                     |
| [number](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Data_structures#number_type)               | [float](https://docs.python.org/3/library/functions.html#float) or [int](https://docs.python.org/3/library/functions.html#int) | `3.1`, `10`                       |                                     |
| [boolean](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Data_structures#boolean_type)             | [bool](https://docs.python.org/3/library/functions.html#bool)                                                                  | `True`, `False`                   |                                     |
| [string](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Data_structures#string_type)               | [str](https://docs.python.org/3/library/stdtypes.html#str)                                                                     | `'abc'`                           |                                     |
| [ArrayBuffer](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/ArrayBuffer) | [bytes](https://docs.python.org/3/library/stdtypes.html#bytes)                                                                 | `b'abc'`                          | ArrayBuffer                         |
| [Array](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Array)             | [list](https://docs.python.org/3/library/stdtypes.html#list)                                                                   | `[1, 3.2, "abc"]`                 | tuple, collections.abc.Sequence     |
| [Set](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Set)                 | ConvexSet (see below)                                                                                                          | `ConvexSet([1,2])`                | set, frozenset, collections.abc.Set |
| [Map](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Map)                 | ConvexMap (see below)                                                                                                          | `ConvexMap([('a', 1), ('b', 2)])` |                                     |
| [object](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Data_structures#objects)                   | [dict](https://docs.python.org/3/library/stdtypes.html#dict)                                                                   | `{a: "abc"}`                      | collections.abc.Mapping             |

### Id

Id objects represent references to Convex documents. They contain a `table_name`
string specifying a Convex table (tables can be viewed in
[the dashboard](https://dashboard.convex.dev)) and a globably unique `id`
string. If you'd like to learn more about the `id` string's format, see
[our docs](https://docs.convex.dev/api/classes/values.GenericId).

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
return JavaScript BigInts will return ConvexBigInt64 instances.

### ConvexSet

Similar to a Python set, but any Convex values can be items.

ConvexSets are returned from Convex cloud function calls that return JavaScript
Sets.

Generally when calling Convex functions from Python, a Python builtin set can be
used instead of a ConvexSet. But for representing unusual types like sets
containing objects, you'll have to use a ConvexSet:

```python
>>> set([{'a': 1}])
Traceback (most recent call last):
    ...
TypeError: unhashable type: 'dict'
>>> ConvexSet([{'a': 1}])
ConvexSet([{'a': 1.0}])
```

ConvexSet instances are immutable so must be fully populated when being
constructed. In order to store mutable items, ConvexSets store snapshots of data
when it was added.

```python
>>> mutable_dict = {'a': 1}
>>> s = ConvexSet([mutable_dict, 'hello', 1])
>>> mutable_dict in s
True
>>> mutable_dict['b'] = 2
>>> mutable_dict in s
False
>>> s
ConvexSet([{'a': 1.0}, 'hello', 1.0])
```

ConvexSets perform a copy of each inserted item, so they require more memory
than Python's builtin sets.

### ConvexMap

Similar to a Python map, but any Convex values can be keys.

ConvexMaps are returned from Convex cloud function calls that return JavaScript
Maps.

ConvexMaps are useful when calling Convex functions that expect a Map because
dictionaries correspond to JavaScript objects, not Maps.

ConvexMap instances are immutable so must be fully populated when being
constructed. In order to store mutable items, ConvexMaps store snapshots of data
when it was added.

```python
>>> mutable_dict = {'a': 1}
>>> s = ConvexMap([(mutable_dict, 123), ('b', 456)])
>>> mutable_dict in s
True
>>> mutable_dict['b'] = 2
>>> mutable_dict in s
False
>>> s
ConvexMap([({'a': 1.0}, 123.0), ('b', 456.0)])
```

ConvexMaps perform a copy of each inserted key/value pair, so they require more
memory than Python's builtin dictionaries.

# Versioning

While we are pre-1.0.0, we'll update the minor version for large changes, and
the patch version for small bugfixes. We may make backwards incompatible changes
to the python client's API, but we will limit those to minor version bumps.
