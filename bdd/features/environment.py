import os
import shutil
import signal


def after_scenario(context, scenario):
    if hasattr(context, "port_guard") and context.port_guard is not None:
        context.port_guard.close()
        context.port_guard = None
    if hasattr(context, "server_process"):
        try:
            os.killpg(context.server_process.pid, signal.SIGTERM)
            context.server_process.wait(timeout=5)
        except:
            try:
                os.killpg(context.server_process.pid, signal.SIGKILL)
            except:
                context.server_process.kill()
            context.server_process.wait(timeout=5)
    if hasattr(context, "temp_dir") and os.path.exists(context.temp_dir):
        shutil.rmtree(context.temp_dir)
