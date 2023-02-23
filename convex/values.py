import base64
import collections
import dataclasses
import json
import math
import re
import struct
import sys
from collections import abc
from typing import (
    TYPE_CHECKING,
    Any,
    Dict,
    Iterable,
    Iterator,
    List,
    Set,
    Tuple,
    Union,
    cast,
)

if sys.version_info[1] < 9:
    # In Python 3.8, abc.* types don't implement __getitem__
    # We don't typecheck in 3.8, so we shim just needs work at runtime.
    # Use abc when indexing types and collections.abc otherwise.
    orig_abc = abc

    class ReplacementAbc:
        def __getattr__(self, attr):
            return Indexable(getattr(orig_abc, attr))

    class Indexable:
        def __init__(self, target):
            self.target = target

        def __getitem__(self, _key):
            return self.target

    abc = ReplacementAbc()

if TYPE_CHECKING:
    from typing_extensions import TypeGuard


__all__ = [
    "Id",
    "JsonValue",
    "ConvexValue",
    "convex_to_json",
    "strict_convex_to_json",
    "json_to_convex",
    "validate_object_field",
]

JsonValue = Union[
    None, bool, str, float, int, List["JsonValue"], Dict[str, "JsonValue"]
]

ConvexValue = Union[
    None,
    bool,
    str,
    float,
    List["ConvexValue"],
    Dict[str, "ConvexValue"],
    "Id",
    Set["ConvexValue"],
    Dict,
    bytes,
    "ConvexSet",
    "ConvexMap",
    "ConvexInt64",
]

# This should be a wider type: it also includes objects that implement the
# buffer protocol.
CoercibleToConvexValue = Union[
    ConvexValue,
    int,
    abc.Mapping["CoercibleToConvexValue", "CoercibleToConvexValue"],
    abc.Sequence["CoercibleToConvexValue"],
    abc.Set["CoercibleToConvexValue"],
]


MAX_IDENTIFIER_LEN = 64
ALL_UNDERSCORES = re.compile("^_+$")
IDENTIFIER_REGEX = re.compile("^[a-zA-Z_][a-zA-Z0-9_]{0,63}$")

MIN_INT64 = -(2**63)
MAX_INT64 = 2**63 - 1

# This is the range of integers safely representable in a float64.
# `Number.MIN_SAFE_INTEGER` and `Number.MAX_SAFE_INTEGER` in JavaScript are one
# closer to zero, but these 2 extra values can be safely serialized.
MIN_SAFE_INTEGER = -(2**53)
MAX_SAFE_INTEGER = 2**53


def int_to_float(v: int) -> float:
    if not MIN_SAFE_INTEGER <= v <= MAX_SAFE_INTEGER:
        raise ValueError(
            f"Integer {v} is outside the range of a Convex `Float64` (-2^53 to 2^53). "
            "Consider using a `ConvexInt64`, which corresponds to a `BigInt` in "
            "JavaScript Convex functions."
        )
    return float(v)


def validate_object_field(k: str) -> None:
    if len(k) == 0:
        raise ValueError("Empty field names are disallowed.")
    if len(k) > MAX_IDENTIFIER_LEN:
        raise ValueError(
            f"Field name {k} exceeds maximum field name length {MAX_IDENTIFIER_LEN}."
        )
    if k.startswith("$"):
        raise ValueError(f"Field name {k} starts with a '$', which is reserved.")
    if ALL_UNDERSCORES.match(k):
        raise ValueError(f"Field name {k} can't exclusively be underscores.")

    if not IDENTIFIER_REGEX.match(k):
        raise ValueError(
            f"Field name {k} must only contain alphanumeric characters or underscores "
            "and can't start with a number."
        )


@dataclasses.dataclass
class Id:
    """
    Id objects represent references to Convex documents. They contain a `table_name`
    string specifying a Convex table (tables can be viewed in
    [the dashboard](https://dashboard.convex.dev)) and a globably unique `id`
    string. If you'd like to learn more about the `id` string's format, see
    [our docs](https://docs.convex.dev/api/classes/values.GenericId).
    """

    table_name: str
    id: str

    @classmethod
    def from_json(cls, obj: Any) -> "Id":
        if not isinstance(obj["$id"], str):
            raise ValueError(f"Object {obj} isn't a valid Id: $id isn't a string.")
        parts = obj["$id"].split("|")
        if len(parts) != 2:
            raise ValueError(f"Object {obj} isn't a valid Id: Wrong number of parts.")
        return Id(parts[0], parts[1])

    def to_json(self) -> JsonValue:
        idString = f"{self.table_name}|{self.id}"
        return {"$id": idString}


def is_special_float(v: float) -> bool:
    """Some values can't be serialized to JSON: return True if this is one of those."""
    return (
        math.isnan(v) or not math.isfinite(v) or (v == 0 and math.copysign(1, v) == -1)
    )


def float_to_json(v: float) -> JsonValue:
    if is_special_float(v):
        data = struct.pack("<d", v)
        return {"$float": base64.standard_b64encode(data).decode("ascii")}
    return v


def mapping_to_object_json(v: abc.Mapping[Any, Any], coerce: bool) -> JsonValue:
    d: Dict[str, JsonValue] = {}
    for key, val in v.items():
        if not isinstance(key, str):
            raise ValueError(f"Convex object keys must be strings, found {key}")
        validate_object_field(key)
        obj_val: JsonValue = _convex_to_json(val, coerce)
        d[key] = obj_val
    return d


def mapping_to_map_json(
    v: abc.Mapping[CoercibleToConvexValue, CoercibleToConvexValue], coerce: bool
) -> JsonValue:
    pairs: List[JsonValue] = []
    for key, val in v.items():
        p1: JsonValue = _convex_to_json(key, coerce)
        p2: JsonValue = _convex_to_json(val, coerce)
        pair: List[JsonValue] = [p1, p2]
        pairs.append(pair)
    return {"$map": pairs}


def buffer_to_json(v: Any) -> JsonValue:
    return {"$bytes": base64.standard_b64encode(v).decode("ascii")}


def iterable_to_array_json(v: abc.Iterable[Any], coerce: bool) -> JsonValue:
    # Convex arrays can have 1024 items maximum.
    # Let the server check this for now.
    return [_convex_to_json(x, coerce) for x in v]


def iterable_to_set_json(v: abc.Iterable[Any], coerce: bool) -> JsonValue:
    # Convex sets can have 1024 items maximum.
    # Let the server check this for now.
    return {"$set": [_convex_to_json(x, coerce) for x in v]}


def convex_to_json(v: CoercibleToConvexValue) -> JsonValue:
    """Convert Convex-serializable values to JSON-serializable objects.

    Convex types are described at https://docs.convex.dev/using/types and
    include Python builtin types str, int, float, bool, bytes, None, list, and
    dict; as well as instances of the Id, ConvexSet, and ConvexMap classes.

    >>> convex_to_json({'a': 1.0})
    {'a': 1.0}
    >>> convex_to_json(Id("messages", "mqMw7arHuQa8TWcCXl8faAW"))
    {'$id': 'messages|mqMw7arHuQa8TWcCXl8faAW'}

    In addition to these basic Convex values, many Python types can be coerced
    to Convex values: for example, builtin sets:

    >>> convex_to_json(set([1.0, 2.0, 3.0]))
    {'$set': [1.0, 2.0, 3.0]}

    These coerced values will be different when roundtripped:

    >>> json_to_convex(convex_to_json(set([1.0, 2.0, 3.0])))
    ConvexSet([1.0, 2.0, 3.0])

    While Python plays fast and loose with ints and floats (divide one int by
    another, get a float!), they correspond to two different types in Convex
    functions: JavaScript numbers (Python float) and JavaScript bigints (Python
    int). To ensure Convex functions receive the correct type (typically float),
    you may want to cast inputs from int to float.
    """
    return _convex_to_json(v, coerce=True)


def strict_convex_to_json(v: ConvexValue) -> JsonValue:
    """Convert Convex round-trippable values to JSON-serializable objects."""
    return _convex_to_json(v, coerce=False)


# There's a server-enforced limit on total document size of 1MB. This could be
# enforced for each field individually, but it could still exceed the document
# size limit when combined, so let the server enforce this.
def _convex_to_json(v: CoercibleToConvexValue, coerce: bool) -> JsonValue:
    # 1. values which roundtrip
    if v is None:
        return None
    if v is True or v is False:
        return v
    if type(v) is float:
        return float_to_json(v)
    if type(v) is str:
        return v
    if type(v) is bytes:
        return buffer_to_json(v)
    if type(v) is dict:
        return mapping_to_object_json(v, coerce)
    if type(v) is list:
        return iterable_to_array_json(v, coerce)
    if type(v) is Id:
        return v.to_json()
    if type(v) is ConvexSet:
        return v.to_json()
    if type(v) is ConvexMap:
        return v.to_json()
    if type(v) is ConvexInt64:
        return v.to_json()

    if not coerce:
        raise TypeError(
            f"{v} is not a supported Convex type. "
            "To learn about Convex's supported types "
            "see https://docs.convex.dev/using/types."
        )

    # 2. common types that don't roundtrip but have clear representations in Convex
    if isinstance(v, int):
        return int_to_float(v)
    if isinstance(v, tuple):
        return iterable_to_array_json(v, coerce)
    if isinstance(v, set):
        return iterable_to_set_json(v, coerce)
    if isinstance(v, frozenset):
        return iterable_to_set_json(v, coerce)

    # 3. allow subclasses (which will not round-trip)
    if isinstance(v, float):
        return float_to_json(v)
    if isinstance(v, str):
        return v
    if isinstance(v, bytes):
        return buffer_to_json(v)
    if isinstance(v, dict):
        return mapping_to_object_json(v, coerce)
    if isinstance(v, Id):
        return v.to_json()
    if isinstance(v, list):
        return iterable_to_array_json(v, coerce)
    if isinstance(v, ConvexSet):
        return v.to_json()
    if isinstance(v, ConvexMap):
        return v.to_json()

    # 4. check for implementing abstract classes and protocols
    try:
        # Does this object conform to the buffer protocol?
        memoryview(v)  # type: ignore
    except TypeError:
        pass
    else:
        return buffer_to_json(v)

    if isinstance(v, collections.abc.Mapping):
        return mapping_to_object_json(v, coerce)
    if isinstance(v, collections.abc.Set):
        return iterable_to_set_json(v, coerce)
    if isinstance(v, collections.abc.Sequence):
        return iterable_to_array_json(v, coerce)

    raise TypeError(
        f"{v} is not a supported Convex type. "
        "To learn about Convex's supported types, see https://docs.convex.dev/using/types."
    )


def json_to_convex(v: JsonValue) -> ConvexValue:
    "Convert from simple Python JSON objects to richer types."

    if isinstance(v, (bool, float, str)):
        return v
    if v is None:
        return None
    if isinstance(v, list):
        convex_values: ConvexValue = [json_to_convex(x) for x in v]
        return convex_values
    if isinstance(v, dict) and len(v) == 1:
        attr = list(v.keys())[0]
        if attr == "$id":
            return Id.from_json(v)
        if attr == "$bytes":
            data_str = cast(str, v["$bytes"])
            return base64.standard_b64decode(data_str)
        if attr == "$integer":
            data_str = cast(str, v["$integer"])
            (i,) = struct.unpack("<q", base64.standard_b64decode(data_str))
            return ConvexInt64(cast(int, i))
        if attr == "$float":
            data_str = cast(str, v["$float"])
            (f,) = struct.unpack("<d", base64.standard_b64decode(data_str))
            if not is_special_float(f):
                raise ValueError("Not a special float: {f}")
            return cast(float, f)
        if attr == "$set":
            return ConvexSet.from_json(v)
        if attr == "$map":
            return ConvexMap.from_json(v)
        if attr.startswith("$"):
            raise ValueError(f"Bad JSON value: {v}")
    if isinstance(v, dict):
        output = {}
        for attr, value in v.items():
            # Currently the only attributes that start with an underscore
            # are _id and _creationTime, but more may be added in the future.
            validate_object_field(attr)
            output[attr] = value
        return {k: json_to_convex(v) for k, v in v.items()}
    raise ValueError(f"Bad JSON value: {v}")


# used for testing
def _json_string_to_convex(s: str) -> ConvexValue:
    return json_to_convex(json.loads(s))


# used for testing
def _convex_to_json_string(v: CoercibleToConvexValue) -> str:
    return json.dumps(_convex_to_json(v, False))


def is_coercible_to_convex_value(v: Any) -> "TypeGuard[CoercibleToConvexValue]":
    """
    >>> is_coercible_to_convex_value(set([1,2,3]))
    True
    """
    try:
        convex_to_json(v)
    except (TypeError, ValueError):
        return False
    return True


def is_convex_value(v: Any) -> "TypeGuard[ConvexValue]":
    """
    >>> is_convex_value(set([1,2,3]))
    False
    """
    try:
        strict_convex_to_json(v)
    except (TypeError, ValueError):
        return False
    return True


class ConvexInt64:
    """
    A wrapper around a Python int.

    >>> ConvexInt64(123)
    ConvexInt64(123)
    """

    def __init__(self, value: int):
        if not isinstance(value, int):
            raise TypeError(f"{value} is not an int")
        if not MIN_INT64 <= value <= MAX_INT64:
            raise ValueError(f"{value} is outside allowed range for an int64")
        self.value = value

    def __repr__(self) -> str:
        return f"ConvexInt64({self.value})"

    def to_json(self) -> JsonValue:
        if not MIN_INT64 <= self.value <= MAX_INT64:
            raise ValueError(f"{self.value} does not fit in an Int64")
        data = struct.pack("<q", self.value)
        return {"$integer": base64.standard_b64encode(data).decode("ascii")}

    def __eq__(self, other: Any) -> bool:
        if type(other) is not ConvexInt64:
            return cast(bool, other == self.value)
        return other.value == self.value


# This docstring also exists in the README, it should match there.
class ConvexSet(abc.Set[ConvexValue]):
    """
    Similar to a Python set, but any Convex values can be items.

    ConvexSets are returned from Convex cloud function calls that return
    JavaScript Sets.

    Generally when calling Convex functions from Python, a Python builtin
    set can be used instead of a ConvexSet.
    But for representing unusual types like sets containing objects, you'll have to use a ConvexSet:

    >>> set([{'a': 1}])
    Traceback (most recent call last):
        ...
    TypeError: unhashable type: 'dict'
    >>> ConvexSet([{'a': 1}])
    ConvexSet([{'a': 1.0}])

    ConvexSet instances are immutable so must be fully populated when being
    constructed. In order to store mutable items, ConvexSets store snapshots
    of data when it was added.

    >>> mutable_dict = {'a': 1}
    >>> s = ConvexSet([mutable_dict, 'hello', 1])
    >>> mutable_dict in s
    True
    >>> mutable_dict['b'] = 2
    >>> mutable_dict in s
    False
    >>> s
    ConvexSet([{'a': 1.0}, 'hello', 1.0])

    ConvexSets perform a copy of each inserted item, so they require more memory
    than Python's builtin sets.
    """

    def __init__(self, items: Iterable[CoercibleToConvexValue]) -> None:
        self._elements: Dict[str, ConvexValue] = {}
        self._ordered: List[Tuple[str, ConvexValue]] = []
        ConvexSet._initialize(self, (convex_to_json(item) for item in items), False)

    @staticmethod
    def from_json(data: JsonValue) -> "ConvexSet":
        """Build a ConvexSet from its JSON representation."""
        if type(data) is not dict or len(data) != 1:
            raise ValueError("Invalid json objects for ConvexSet")
        (attr,) = data.keys()
        if attr != "$set":
            raise ValueError("Invalid json objects for ConvexSet")

        s = ConvexSet([])
        items_data = cast(List[JsonValue], data["$set"])
        ConvexSet._initialize(s, items_data, True)
        return s

    @staticmethod
    def _initialize(
        s: "ConvexSet", json_items: Iterable[JsonValue], preserve_order: bool
    ) -> None:
        for json_item in json_items:
            hash = json.dumps(json_item)
            obj_item = json_to_convex(json_item)
            if hash in s._elements:
                raise ValueError(f"Duplicate value in ConvexSet: {obj_item!r}")
            s._elements[hash] = obj_item
            s._ordered.append((hash, obj_item))

        if not preserve_order:
            s._ordered = sorted(s._elements.items())

    def to_json(self) -> JsonValue:
        # Convex sets can have 1024 items maximum.
        # Let the server check this for now.
        return {"$set": [convex_to_json(v) for _, v in self._ordered]}

    # collections.abc.Set methods
    def __contains__(self, item: object) -> bool:
        try:
            json_object = convex_to_json(cast(CoercibleToConvexValue, item))
        except (ValueError, TypeError):
            return False
        hash = json.dumps(json_object)
        return hash in self._elements

    def __iter__(self) -> Iterator[ConvexValue]:
        return (el for _hash, el in self._ordered)

    def __len__(self) -> int:
        return len(self._ordered)

    # bonus methods
    def __eq__(self, other: Any) -> bool:
        if type(other) is not ConvexSet:
            return False
        return other._elements == self._elements

    def __repr__(self) -> str:
        return (
            f"ConvexSet([{', '.join(repr(el) for hash, el in self._elements.items())}])"
        )


# This docstring also exists in the README, it should match there.
class ConvexMap(abc.Mapping[ConvexValue, ConvexValue]):
    """
    Similar to a Python map, but any Convex values can be keys.

    ConvexMaps are returned from Convex cloud function calls that return
    JavaScript Maps.

    ConvexMaps are useful when calling Convex functions that expect a Map
    because dictionaries correspond to JavaScript objects, not Maps.

    ConvexMap instances are immutable so must be fully populated when being
    constructed. In order to store mutable items, ConvexMaps store snapshots
    of data when it was added.

    >>> mutable_dict = {'a': 1}
    >>> s = ConvexMap([(mutable_dict, 123), ('b', 456)])
    >>> mutable_dict in s
    True
    >>> mutable_dict['b'] = 2
    >>> mutable_dict in s
    False
    >>> s
    ConvexMap([({'a': 1.0}, 123.0), ('b', 456.0)])

    ConvexMaps perform a copy of each inserted key/value pair, so they require more
    memory than Python's builtin dictionaries.
    """

    def __init__(
        self, items: Iterable[Tuple[CoercibleToConvexValue, CoercibleToConvexValue]]
    ) -> None:
        self._kv_pairs: Dict[str, Tuple[ConvexValue, ConvexValue]] = {}
        self._ordered: List[Tuple[str, Tuple[ConvexValue, ConvexValue]]] = []
        ConvexMap._initialize(
            self, ((convex_to_json(k), convex_to_json(v)) for k, v in items), False
        )

    @staticmethod
    def from_json(data: JsonValue) -> "ConvexMap":
        """Build a ConvexMap from its JSON representation."""
        if type(data) is not dict or len(data) != 1:
            raise ValueError("Invalid json objects for ConvexMap")
        (attr,) = data.keys()
        if attr != "$map":
            raise ValueError("Invalid json objects for ConvexMap")

        s = ConvexMap([])
        kv_data = cast(List[Tuple[JsonValue, JsonValue]], data["$map"])
        ConvexMap._initialize(s, kv_data, True)
        return s

    @staticmethod
    def _initialize(
        s: "ConvexMap",
        json_items: Iterable[Tuple[JsonValue, JsonValue]],
        preserve_order: bool,
    ) -> None:
        for json_key, json_value in json_items:
            hash = json.dumps(json_key)
            obj_key = json_to_convex(json_key)
            if hash in s._kv_pairs:
                raise ValueError(f"Duplicate key in ConvexMap: {obj_key!r}")
            obj_value = json_to_convex(json_value)
            pair = (obj_key, obj_value)
            s._kv_pairs[hash] = pair
            s._ordered.append((hash, pair))

        if not preserve_order:
            s._ordered = sorted(s._kv_pairs.items())

    def to_json(self) -> JsonValue:
        # Convex maps can have 1024 items maximum.
        # Let the server check this for now.
        return {
            "$map": [
                [convex_to_json(k), convex_to_json(v)] for _, (k, v) in self._ordered
            ]
        }

    # collections.abc.Mapping methods
    def __getitem__(self, key: CoercibleToConvexValue) -> ConvexValue:
        hash = json.dumps(convex_to_json(key))
        if hash not in self._kv_pairs:
            raise KeyError(f"Key {key!r} not in ConvexMap")
        return self._kv_pairs[hash][1]

    def __iter__(self) -> Iterator[ConvexValue]:
        return (k for _, (k, _) in self._ordered)

    def __len__(self) -> int:
        return len(self._ordered)

    # bonus methods
    def __eq__(self, other: Any) -> bool:
        if type(other) is not ConvexMap:
            return False
        return other._kv_pairs == self._kv_pairs

    def __repr__(self) -> str:
        return f"ConvexMap([{', '.join(repr((k, v)) for hash, (k, v) in self._kv_pairs.items())}])"
