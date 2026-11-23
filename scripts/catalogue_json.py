"""JSON output byte-compatible with `JSON.stringify(value, null, 2)`.

The committed catalogue snapshots were written by JavaScript tooling. Python's
encoder differs in how it prints whole-number floats (`1.0` versus `1`) and in
its exponent thresholds, so the importers use this serializer to keep the
snapshots (and their drift checks) stable. It also reproduces JavaScript's
property order, which lists integer-like keys before other keys.
"""

from __future__ import annotations

from decimal import Decimal
import json
import math


def number(value: float) -> str:
    """The ECMAScript Number-to-string conversion for a finite float."""
    if value == 0:
        return "0"
    if not math.isfinite(value):
        return "null"  # JSON.stringify prints NaN and Infinity as null
    sign = "-" if value < 0 else ""
    sign_bit, digit_tuple, exponent = Decimal(repr(abs(value))).as_tuple()
    digits = "".join(map(str, digit_tuple)).rstrip("0") or "0"
    # value = 0.DIGITS x 10^point
    point = len(digit_tuple) + exponent
    count = len(digits)
    if count <= point <= 21:
        return f"{sign}{digits}{'0' * (point - count)}"
    if 0 < point <= 21:
        return f"{sign}{digits[:point]}.{digits[point:]}"
    if -6 < point <= 0:
        return f"{sign}0.{'0' * -point}{digits}"
    shifted = point - 1
    mantissa = digits if count == 1 else f"{digits[0]}.{digits[1:]}"
    return f"{sign}{mantissa}e{'+' if shifted >= 0 else '-'}{abs(shifted)}"


def _is_index(key: str) -> bool:
    return key.isascii() and key.isdigit() and (key == "0" or key[0] != "0") and int(key) < 2**32 - 1


def js_key_order(keys) -> list[str]:
    """JavaScript objects list integer-like keys first, ascending, then the rest in insertion order."""
    keys = list(keys)
    integers = sorted((key for key in keys if _is_index(key)), key=int)
    return integers + [key for key in keys if not _is_index(key)]


def _string(value):
    # JSON.stringify escapes lone surrogates while leaving ordinary Unicode intact.
    return ''.join(f'\\u{ord(c):04x}' if 0xD800 <= ord(c) <= 0xDFFF else c
                   for c in json.dumps(value, ensure_ascii=False))


def _encode(value, indent: str, out: list[str], step: str | None) -> None:
    if value is None:
        out.append("null")
    elif value is True:
        out.append("true")
    elif value is False:
        out.append("false")
    elif isinstance(value, int):
        out.append(number(float(value)))
    elif isinstance(value, float):
        out.append(number(value))
    elif isinstance(value, str):
        out.append(_string(value))
    elif isinstance(value, (list, tuple)):
        if not value:
            out.append("[]")
            return
        if step is None:
            out.append("[" + ",".join(dumps(item, indent=None) for item in value) + "]")
            return
        inner = indent + step
        out.append("[\n")
        for index, item in enumerate(value):
            out.append(inner)
            _encode(item, inner, out, step)
            out.append(",\n" if index < len(value) - 1 else "\n")
        out.append(indent + "]")
    elif isinstance(value, dict):
        if not value:
            out.append("{}")
            return
        if step is None:
            out.append("{" + ",".join(_string(key) + ":" + dumps(value[key], indent=None)
                                      for key in js_key_order(value)) + "}")
            return
        inner = indent + step
        out.append("{\n")
        for index, key in enumerate(js_key_order(value)):
            out.append(inner + _string(key) + ": ")
            _encode(value[key], inner, out, step)
            out.append(",\n" if index < len(value) - 1 else "\n")
        out.append(indent + "}")
    else:
        raise TypeError(f"Cannot serialize {type(value).__name__}")


def dumps(value, indent="  ") -> str:
    out: list[str] = []
    _encode(value, "", out, indent)
    return "".join(out)
