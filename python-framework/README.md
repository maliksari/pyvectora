# PyVectora

**Rust-powered Python framework for high-performance APIs, AI and microservices.**

## Installation

```bash
pip install pyvectora
```

## Quick Start

```python
from pyvectora import App, Response

app = App()

@app.get("/")
def index(request):
    return Response.json({"message": "Hello, World!"})

@app.get("/hello/{name}")
def hello(request):
    name = request.params.get("name", "World")
    return Response.json({"message": f"Hello, {name}!"})

app.serve()
```

## Features

- 🚀 **Blazing Fast** - Rust-powered HTTP server using Hyper & Tokio
- 🐍 **Pythonic API** - Familiar decorator-based routing like Flask/FastAPI
- ⚡ **Zero-Copy** - Efficient memory handling via Rust FFI
- 🔒 **Type Safe** - Full type hints and Pydantic integration
- 🌐 **Production Ready** - Graceful shutdown, logging, error handling

## Development

```bash
# Build native module
cd python-framework
maturin develop

# Run example
cd ../examples/hello_api
python main.py
```

## License

Apache-2.0
