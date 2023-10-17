"""Value types supported by Convex."""
import base64
import collections
import json
import math
import struct
import sys
from collections import abc
from typing import TYPE_CHECKING, Any, Dict, List, Union, cast

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
    "JsonValue",
    "ConvexValue",
    "convex_to_json",
    "strict_convex_to_json",
    "json_to_convex",
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
    Dict,
    bytes,
    "ConvexInt64",
]

# This should be a wider type: it also includes objects that implement the
# buffer protocol.
CoercibleToConvexValue = Union[
    ConvexValue,
    int,
    abc.Mapping["CoercibleToConvexValue", "CoercibleToConvexValue"],
    abc.Sequence["CoercibleToConvexValue"],
]


class ConvexError(Exception):
    """Custom ConvexError to handle custom application-level errors."""

    def __init__(self, data: ConvexValue):
        """Instantiate a ConvexError from a ConvexValue that is passed in."""
        super().__init__(
            data if isinstance(data, str) else _convex_to_json_string(data)
        )
        self.data = data
        self._a = True


MAX_IDENTIFIER_LEN = 1024

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


def _validate_object_field(k: str) -> None:
    if len(k) == 0:
        raise ValueError("Empty field names are disallowed.")
    if len(k) > MAX_IDENTIFIER_LEN:
        raise ValueError(
            f"Field name {k} exceeds maximum field name length {MAX_IDENTIFIER_LEN}."
        )
    if k.startswith("$"):
        raise ValueError(f"Field name {k} starts with a '$', which is reserved.")

    for char in k:
        # Non-control ASCII characters
        if ord(char) < 32 or ord(char) >= 127:
            raise ValueError(
                f"Field name '{k}' has invalid character '{char}': "
                "Field names can only contain non-control ASCII characters"
            )


def is_special_float(v: float) -> bool:
    """Return True if value cannot be serialized to JSON."""
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
        _validate_object_field(key)
        obj_val: JsonValue = _convex_to_json(val, coerce)
        d[key] = obj_val
    return d


def buffer_to_json(v: Any) -> JsonValue:
    return {"$bytes": base64.standard_b64encode(v).decode("ascii")}


def iterable_to_array_json(v: abc.Iterable[Any], coerce: bool) -> JsonValue:
    # Convex arrays can have 8192 items maximum.
    # Let the server check this for now.
    return [_convex_to_json(x, coerce) for x in v]


def convex_to_json(v: CoercibleToConvexValue) -> JsonValue:
    """Convert Convex-serializable values to JSON-serializable objects.

    Convex types are described at https://docs.convex.dev/using/types and
    include Python builtin types str, int, float, bool, bytes, None, list, and
    dict, as well as instances of the ConvexInt64 class.

    >>> convex_to_json({'a': 1.0})
    {'a': 1.0}

    In addition to these basic Convex values, many Python types can be coerced
    to Convex values: for example, tuples:

    >>> convex_to_json((1.0, 2.0, 3.0))
    [1.0, 2.0, 3.0]

    Python plays fast and loose with ints and floats (divide one int by another,
    get a float!), which makes treating these as two separate types in Convex
    functions awkward. Convex functions run in JavaScript, where `number` (an
    IEEE 754 double-precision float) is frequently used for both ints and
    floats.
    To ensure Convex functions receive floats, both ints and floats in Python are
    converted to floats.

    >>> json_to_convex(convex_to_json(1.23))
    1.23
    >>> json_to_convex(convex_to_json(17))
    17.0

    Convex supports storing Int64s in the database, represented in JavaScript as
    BigInts. To specify an Int64, use the ConvexInt64 wrapper type.

    >>> json_to_convex(convex_to_json(ConvexInt64(17)))
    ConvexInt64(17)
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

    # 3. allow subclasses (which will not round-trip)
    if isinstance(v, float):
        return float_to_json(v)
    if isinstance(v, str):
        return v
    if isinstance(v, bytes):
        return buffer_to_json(v)
    if isinstance(v, dict):
        return mapping_to_object_json(v, coerce)
    if isinstance(v, list):
        return iterable_to_array_json(v, coerce)

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
    if isinstance(v, collections.abc.Sequence):
        return iterable_to_array_json(v, coerce)

    raise TypeError(
        f"{v} is not a supported Convex type. "
        "To learn about Convex's supported types, see https://docs.convex.dev/using/types."
    )


def json_to_convex(v: JsonValue) -> ConvexValue:
    """Convert from simple Python JSON objects to richer types."""
    if isinstance(v, (bool, float, str)):
        return v
    if v is None:
        return None
    if isinstance(v, list):
        convex_values: ConvexValue = [json_to_convex(x) for x in v]
        return convex_values
    if isinstance(v, dict) and len(v) == 1:
        attr = list(v.keys())[0]
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
        if attr.startswith("$"):
            raise ValueError(f"Bad JSON value: {v}")
    if isinstance(v, dict):
        output = {}
        for attr, value in v.items():
            # Currently the only attributes that start with an underscore
            # are _id and _creationTime, but more may be added in the future.
            _validate_object_field(attr)
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
    """Return True if value is coercible to a convex value.

    >>> is_coercible_to_convex_value((1,2,3))
    True
    """
    try:
        convex_to_json(v)
    except (TypeError, ValueError):
        return False
    return True


def is_convex_value(v: Any) -> "TypeGuard[ConvexValue]":
    """Return True if value is a convex value.

    >>> is_convex_value((1,2,3))
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
