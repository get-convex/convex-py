# _convex.pyi

# These types are defined in `crates/py_client/src/client/mod.rs` and `crates/py_client/src/client/subscription.rs`.
# Types in this file will need to be manually updated when these pyo3-annotated structs change.

from typing import Any, Awaitable, Dict, Literal, Optional, Union

from typing_extensions import TypedDict

class ValueResult(TypedDict):
    type: Literal["value"]
    value: Any

class ConvexErrorResult(TypedDict):
    type: Literal["convexerror"]
    message: str
    data: Any

Result = Union[ValueResult, ConvexErrorResult]

class PyQuerySubscription:
    def exists(self) -> bool: ...
    @property
    def id(self) -> Any: ...
    def unsubscribe(self) -> None: ...
    def next(self) -> Result: ...
    def anext(self) -> Awaitable[Result]: ...

class PyQuerySetSubscription:
    def exists(self) -> bool: ...
    # TODO Why is this optional?
    def next(self) -> Optional[Dict[Any, Any]]: ...
    # TODO Why is this optional?
    def anext(self) -> Awaitable[Optional[Dict[Any, Any]]]: ...

class PyConvexClient:
    def __new__(cls, deployment_url: str) -> "PyConvexClient": ...
    def subscribe(
        self, name: str, args: Optional[Dict[str, Any]] = None
    ) -> PyQuerySubscription: ...
    def query(self, name: str, args: Optional[Dict[str, Any]] = None) -> Result: ...
    def mutation(self, name: str, args: Optional[Dict[str, Any]] = None) -> Result: ...
    def action(self, name: str, args: Optional[Dict[str, Any]] = None) -> Result: ...
    def watch_all(self) -> PyQuerySetSubscription: ...
    def set_auth(self, token: Optional[str]) -> None: ...
    def set_admin_auth(self, token: str) -> None: ...

def init_logging() -> None: ...
def py_to_rust_to_py(value: Any) -> Any:
    """Convert a Python value to Rust and bring it back to test conversions."""