import os
import signal
import subprocess


def start_server_process(args, **kwargs):
    if (
        kwargs.get("stdout") == subprocess.PIPE
        and kwargs.get("stderr") == subprocess.PIPE
    ):
        kwargs["stderr"] = subprocess.STDOUT
    return subprocess.Popen(args, start_new_session=True, **kwargs)


def stop_server_process(process):
    try:
        os.killpg(process.pid, signal.SIGTERM)
        process.wait(timeout=5)
    except:
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except:
            process.kill()
        process.wait(timeout=5)
