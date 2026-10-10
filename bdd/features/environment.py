import os
import shutil

from server_process import stop_server_process


def after_scenario(context, scenario):
    if hasattr(context, "port_guard") and context.port_guard is not None:
        context.port_guard.close()
        context.port_guard = None
    if hasattr(context, "server_process"):
        stop_server_process(context.server_process)
    if hasattr(context, "server_log_thread"):
        context.server_log_thread.join(timeout=5)
        if context.server_log_thread.is_alive():
            raise TimeoutError("BDD server log stream did not close")
    if hasattr(context, "temp_dir") and os.path.exists(context.temp_dir):
        shutil.rmtree(context.temp_dir)
