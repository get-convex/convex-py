# import { Value, stringifyValueForError } from "./value.js";

# const IDENTIFYING_FIELD = Symbol.for("ConvexError");

# export class ConvexError<TData extends Value> extends Error {
#   name = "ConvexError";
#   data: TData;
#   [IDENTIFYING_FIELD] = true;

#   constructor(data: TData) {
#     super(typeof data === "string" ? data : stringifyValueForError(data));
#     this.data = data;
#   }
# }

# generics dont make sense for 1 element?
# from typing import TypeVar
# T = TypeVar('T')

from convex.values import _convex_to_json_string
from values import ConvexValue

class ConvexError(Exception):
    def __init__(self, data: ConvexValue):
        super().__init__(data if type(data) == "string" else _convex_to_json_string(data))
        self.data = data

