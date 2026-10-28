import os
import shutil
import subprocess
from pathlib import Path

from server_process import stop_server_process


def before_all(context):
    # Several step modules invoke target/debug/xidlc directly. Build the CLI
    # once up front so no shard depends on another scenario having compiled it.
    root = Path(__file__).resolve().parents[2]
    result = subprocess.run(
        ["cargo", "build", "-p", "xidlc", "--features", "cli"],
        cwd=root,
        capture_output=True,
        text=True,
        timeout=900,
    )
    if result.returncode != 0:
        raise RuntimeError(
            f"failed to build the xidlc CLI required by the BDD steps:\n{result.stderr}"
        )


def after_scenario(context, scenario):
    if hasattr(context, "port_guard") and context.port_guard is not None:
        context.port_guard.close()
        context.port_guard = None
    if hasattr(context, "port_v2_guard") and context.port_v2_guard is not None:
        context.port_v2_guard.close()
        context.port_v2_guard = None
    if hasattr(context, "server_process"):
        stop_server_process(context.server_process)
    if hasattr(context, "server_log_thread"):
        context.server_log_thread.join(timeout=5)
        if context.server_log_thread.is_alive():
            raise TimeoutError("BDD server log stream did not close")
    if hasattr(context, "temp_dir") and os.path.exists(context.temp_dir):
        shutil.rmtree(context.temp_dir)
