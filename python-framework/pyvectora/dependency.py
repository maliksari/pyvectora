import inspect
from typing import Any, Callable, Dict, get_type_hints

class Depends:
    """
    Dependency Injection marker.
    """
    def __init__(self, dependency: Callable[..., Any] | None = None, use_cache: bool = True) -> None:
        self.dependency = dependency
        self.use_cache = use_cache

    def __repr__(self) -> str:
        attr = getattr(self.dependency, "__name__", type(self.dependency).__name__)
        return f"Depends({attr})"

async def solve_dependencies(handler: Callable[..., Any], request: Any) -> Dict[str, Any]:
    """
    Recursively resolve dependencies for a handler.

    Args:
        handler: The function to resolve dependencies for.
        request: The current PyRequest object.

    Returns:
        A dictionary of argument names to injected values.
    """
    sig = inspect.signature(handler)
    kwargs = {}

    for name, param in sig.parameters.items():
        if isinstance(param.default, Depends):
            dep_func = param.default.dependency
            if dep_func:
                dep_kwargs = await solve_dependencies(dep_func, request)

                if inspect.iscoroutinefunction(dep_func):
                    result = await dep_func(**dep_kwargs)
                else:
                    result = dep_func(**dep_kwargs)

                kwargs[name] = result
            else:
                pass

        elif name == "request":
             kwargs[name] = request

    return kwargs

def wrap_handler_with_di(handler: Callable[..., Any]) -> Callable[..., Any]:
    """
    Wraps a user handler to automatically inject dependencies.
    """
    async def wrapper(request: Any, *args, **raw_kwargs) -> Any:
        di_kwargs = await solve_dependencies(handler, request)

        path_params = {}
        if hasattr(request, "params"):
            path_params = request.params

        final_kwargs = {**path_params, **raw_kwargs, **di_kwargs}

        if inspect.iscoroutinefunction(handler):
            return await handler(**final_kwargs)
        else:
            return handler(**final_kwargs)

    return wrapper
