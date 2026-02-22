from __future__ import annotations
import json
import dataclasses
from typing import Type, TypeVar, Any, get_type_hints, Dict

T = TypeVar("T", bound="Contract")

@dataclasses.dataclass
class Contract:
    """
    Base class for PyVectora Data Contracts.

    A Contract is a verified data structure used for Request Bodies and Responses.
    It replaces complex Model classes with standard Python `dataclasses`.

    Usage:
        @dataclass
        class CreateUser(Contract):
            username: str
            age: int
    """

    @classmethod
    def from_json(cls: Type[T], json_str: str) -> T:
        """
        Parse JSON string and validate against the Contract schema.

        TODO: In Phase 4, replace this with `pyvectora_core.validate_and_parse(json_str, cls.__name__)`
        for Rust-powered SIMD parsing.
        """
        try:
            data = json.loads(json_str)
        except json.JSONDecodeError as e:
            raise ValueError(f"Invalid JSON: {e}")

        return cls.from_dict(data)

    @classmethod
    def from_dict(cls: Type[T], data: Dict[str, Any]) -> T:
        """
        Validate dictionary against type hints and instantiate.
        """
        if not isinstance(data, dict):
            raise ValueError("Payload must be a JSON object")

        hints = get_type_hints(cls)
        init_args = {}

        errors = []

        for field_name, field_type in hints.items():
            if field_name.startswith("_"): continue

            if field_name not in data:
                field = None
                for f in dataclasses.fields(cls):
                    if f.name == field_name:
                        field = f
                        break

                if field and (field.default is not dataclasses.MISSING or field.default_factory is not dataclasses.MISSING):
                    continue

                errors.append(f"Missing field: {field_name}")
                continue

            value = data[field_name]

            origin = getattr(field_type, "__origin__", None)

            if origin is None:
                if not isinstance(value, field_type):
                    try:
                        value = field_type(value)
                    except (ValueError, TypeError):
                         errors.append(f"Field '{field_name}' must be of type {field_type.__name__}, got {type(value).__name__}")

            init_args[field_name] = value

        if errors:
            raise ValueError(f"Contract Validation Failed: {'; '.join(errors)}")

        return cls(**init_args)

    def to_dict(self) -> Dict[str, Any]:
        return dataclasses.asdict(self)

    def to_json(self) -> str:
        return json.dumps(self.to_dict())
