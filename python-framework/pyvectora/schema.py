from __future__ import annotations
import inspect
from typing import Any, Dict, List, Type, get_type_hints
from dataclasses import fields, is_dataclass
from .app import App
from .contract import Contract

class OpenAPIGenerator:
    """
    Generates OpenAPI 3.1 schema from PyVectora App structure.
    Reflects over Controllers, Routes, and Contracts.
    """

    def __init__(self, app: App, title: str = "PyVectora API", version: str = "1.0.0"):
        self.app = app
        self.title = title
        self.version = version
        self.schemas: Dict[str, Any] = {}

    def generate(self) -> Dict[str, Any]:
        paths: Dict[str, Any] = {}

        for controller in self.app._controllers:
            meta = getattr(controller, "_controller_meta", None)
            if not meta: continue

            tag = meta.tags[0] if meta.tags else controller.__class__.__name__

            for route in meta.routes:
                full_path = self._normalize_path(meta.prefix, route.path)

                method = route.method.lower()

                handler = getattr(controller, route.handler_name)
                sig = inspect.signature(handler)
                hints = get_type_hints(handler)

                operation = {
                    "tags": [tag],
                    "summary": route.handler_name.replace("_", " ").title(),
                    "responses": {"200": {"description": "Successful Response"}}
                }

                request_body = self._resolve_request_body(sig, hints)
                if request_body:
                    operation["requestBody"] = request_body

                paths.setdefault(full_path, {})[method] = operation

        return {
            "openapi": "3.1.0",
            "info": {"title": self.title, "version": self.version},
            "paths": paths,
            "components": {"schemas": self.schemas}
        }

    def _normalize_path(self, prefix: str, path: str) -> str:
        prefix = prefix.rstrip("/")
        if not path.startswith("/"):
            path = "/" + path
        combined = prefix + path
        return combined if combined else "/"

    def _resolve_request_body(self, sig: inspect.Signature, hints: Dict[str, Any]) -> Dict[str, Any] | None:
        for name, param in sig.parameters.items():
            t = hints.get(name)
            if t and isinstance(t, type) and issubclass(t, Contract):
                schema_ref = self._register_schema(t)
                return {
                    "content": {
                        "application/json": {
                            "schema": {"$ref": schema_ref}
                        }
                    },
                    "required": True
                }
        return None

    def _register_schema(self, contract_cls: Type[Contract]) -> str:
        name = contract_cls.__name__
        if name in self.schemas:
            return f"#/components/schemas/{name}"

        properties = {}
        required = []

        if is_dataclass(contract_cls):
            type_hints = get_type_hints(contract_cls)
            for field in fields(contract_cls):
                field_type = type_hints.get(field.name)
                prop_schema = self._type_to_schema(field_type)
                properties[field.name] = prop_schema
                required.append(field.name)

        self.schemas[name] = {
            "type": "object",
            "properties": properties,
            "required": required,
            "title": name
        }

        return f"#/components/schemas/{name}"

    def _type_to_schema(self, t: Type) -> Dict[str, Any]:
        if t == str: return {"type": "string"}
        if t == int: return {"type": "integer"}
        if t == float: return {"type": "number"}
        if t == bool: return {"type": "boolean"}
        return {"type": "string"} # Fallback
