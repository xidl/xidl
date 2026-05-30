import os
import shutil
import signal

def after_scenario(context, scenario):
    if hasattr(context, "server_process"):
        try:
            os.kill(context.server_process.pid, signal.SIGTERM)
            context.server_process.wait(timeout=5)
        except:
            context.server_process.kill()
    if hasattr(context, "temp_dir") and os.path.exists(context.temp_dir):
        shutil.rmtree(context.temp_dir)
