import collections
import json
from typing import Any, Optional

import pytest

from convex.values import (
    CoercibleToConvexValue,
    ConvexValue,
    convex_to_json,
    json_to_convex,
    strict_convex_to_json,
)


# These tests check that values produced by this library roundtrip.
# It's even more important to check that values from the backend roundtrip,
# which is tested elsewhere.
def coerced_roundtrip(original: CoercibleToConvexValue) -> None:
    """Assert that a coercible to Python value roundtrips to Convex types.

    - can be coerced to a Convex value (this may NOT roundtrip)
    - has a json representation that roundtrips through a Convex value
    - has a coerced Convex value that roundtrips through JSON representation
    """
    # this coerced value may not be equal to the original
    coerced = json_to_convex(json.loads(json.dumps(convex_to_json(original))))
    strict_roundtrip(coerced)


def strict_roundtrip(original: ConvexValue) -> None:
    """Assert that a Python value roundtrips to Convex types.

    - roundtrips through json, and
    - has a json object representation that also roundtrips
    """
    json_object = strict_convex_to_json(original)

    # the json itself should roundtrip through serialization
    assert json_object == json.loads(json.dumps(json_object))

    roundtripped_object = json_to_convex(json_object)

    # the original object should roundtrip
    assert roundtripped_object == original

    roundtripped_json_object = strict_convex_to_json(roundtripped_object)

    # the json object representation of it should also roundtrip
    assert roundtripped_json_object == json_object
    # ...even when serialized
    assert json.dumps(roundtripped_json_object) == json.dumps(json_object)


def coerced_roundtrip_raises(
    original: Any, message: Optional[str] = None
) -> pytest.ExceptionInfo[Exception]:
    with pytest.raises((ValueError, TypeError)) as e:
        convex_to_json(original)
    if message:
        assert message in e.value.args[0]
    return e


def test_strict_values() -> None:
    strict_roundtrip(None)
    strict_roundtrip(0.123)
    strict_roundtrip("abc")
    strict_roundtrip({"a": 0.123})
    strict_roundtrip({})
    strict_roundtrip({"a": 1.0, "b": 2.0})
    strict_roundtrip(b"abc")

    # special values
    strict_roundtrip(float("inf"))
    strict_roundtrip(float("-0.0"))


def test_subclassed_values() -> None:
    class IntSubclass(int):
        pass

    coerced_roundtrip(IntSubclass(0))

    class FloatSubclass(float):
        pass

    coerced_roundtrip(FloatSubclass(123.0))

    class StrSubclass(str):
        pass

    coerced_roundtrip(StrSubclass("asdf"))

    class BytesSubclass(bytes):
        pass

    coerced_roundtrip(BytesSubclass(b"adsf"))

    class DictSubclass(dict):  # type: ignore
        pass

    coerced_roundtrip(DictSubclass(a=1))

    class ListSubclass(list):  # type: ignore
        pass

    coerced_roundtrip(ListSubclass("abc"))


def test_coercion() -> None:
    # integers are coerced to floats
    coerced_roundtrip(0)
    coerced_roundtrip(1)

    coerced_roundtrip((1, 2))
    coerced_roundtrip(range(10))
    coerced_roundtrip(bytearray(b"abc"))
    coerced_roundtrip(collections.Counter("asdf"))


def test_non_values() -> None:
    coerced_roundtrip_raises(object())
    coerced_roundtrip_raises(object)
    coerced_roundtrip_raises(list)
    coerced_roundtrip_raises(pytest)
    coerced_roundtrip_raises(Any)
    coerced_roundtrip_raises(convex_to_json)
    coerced_roundtrip_raises({1: 1})

    # value errors
    coerced_roundtrip(2**53)
    coerced_roundtrip(-(2**53))
    coerced_roundtrip_raises(2**53 + 1)
    coerced_roundtrip_raises(-(2**53 + 1))


def test_context_errors() -> None:
    coerced_roundtrip_raises({"$a": 1}, "starts with a '$'")
    coerced_roundtrip_raises({"b": {2: 1}}, "must be strings")


def test_decode_json() -> None:
    # TODO prevent top-level _a
    with pytest.raises(ValueError) as e:
        json_to_convex({"$a": 1})
    assert "$" in e.value.args[0]
