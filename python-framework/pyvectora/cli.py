
import os
import sys
import typer
import importlib
from watchfiles import run_process
from typing import Optional

app = typer.Typer()

def run_server(app_import: str, host: str, port: int):
    """
    Actually imports and runs the app.
    This function is run by watchfiles in a subprocess.
    """
    sys.path.insert(0, os.getcwd())

    try:
        module_name, obj_name = app_import.split(":")
    except ValueError:
        print(f"Error: Invalid app string '{app_import}'. Format must be 'module:attribute'")
        return

    try:
        module = importlib.import_module(module_name)
        app_obj = getattr(module, obj_name)
    except (ImportError, AttributeError) as e:
        print(f"Error loading app: {e}")
        return

    if callable(app_obj) and not hasattr(app_obj, "serve"):
         print(f"INFO: Calling factory function '{obj_name}'...")
         app_instance = app_obj()
    else:
         app_instance = app_obj

    if hasattr(app_instance, "serve"):
         print(f"INFO: Starting server on http://{host}:{port}")
         app_instance.serve()
    else:
         print(f"Error: '{obj_name}' is not a valid PyVectora App instance.")

@app.command()
def run(
    app_import: str = typer.Argument(..., help="Application import string, e.g. 'main:app'"),
    host: str = typer.Option("127.0.0.1", help="Bind host"),
    port: int = typer.Option(8000, help="Bind port"),
    reload: bool = typer.Option(True, help="Enable auto-reload on file changes"),
):
    """
    Run the PyVectora development server.
    """
    if reload:
        print(f"INFO:  Will watch for changes in {os.getcwd()}")
        run_process(
            os.getcwd(),
            target=run_server,
            args=(app_import, host, port)
        )
    else:
        run_server(app_import, host, port)

if __name__ == "__main__":
    app()
