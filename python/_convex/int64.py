import base64
import struct
from typing import Any, Dict, cast

MIN_INT64 = -(2**63)
MAX_INT64 = 2**63 - 1


# Don't inherit from int to keep this as explicit as possible.
class ConvexInt64:
    """
    A wrapper around a Python int to representing a Convex Int64.

    >>> ConvexInt64(123)
    ConvexInt64(123)
    """

    def __init__(self, value: int):
        """Create a wrapper around a Python int to representing a Convex Int64."""
        if not isinstance(value, int):
            raise TypeError(f"{value} is not an int")
        if not MIN_INT64 <= value <= MAX_INT64:
            raise ValueError(f"{value} is outside allowed range for an int64")
        self.value = value

    def __repr__(self) -> str:
        return f"ConvexInt64({self.value})"

    def to_json(self) -> Dict[str, Any]:
        """Convert this Int64 to its wrapped Convex representation."""
        if not MIN_INT64 <= self.value <= MAX_INT64:
            raise ValueError(f"{self.value} does not fit in an Int64")
        data = struct.pack("<q", self.value)
        return {"$integer": base64.standard_b64encode(data).decode("ascii")}

    def __eq__(self, other: Any) -> bool:
        if type(other) is not ConvexInt64:
            return cast(bool, other == self.value)
        return other.value == self.value
