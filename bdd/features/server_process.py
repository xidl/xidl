import os
import signal
import subprocess
import time


def start_server_process(args, **kwargs):
    if (
        kwargs.get("stdout") == subprocess.PIPE
        and kwargs.get("stderr") == subprocess.PIPE
    ):
        kwargs["stderr"] = subprocess.STDOUT
    return subprocess.Popen(args, start_new_session=True, **kwargs)


def stop_server_process(process, timeout=5):
    # The launcher can exit before its children finish writing (e.g. Next.js).
    # Reap it while waiting for the session's entire process group to disappear.
    permission_error = None
    for sig in (signal.SIGTERM, signal.SIGKILL):
        try:
            os.killpg(process.pid, sig)
        except ProcessLookupError:
            process.wait(timeout=timeout)
            return
        except PermissionError as error:
            # Darwin can report EPERM while a group contains only zombies.
            # It still exists: keep waiting for ESRCH, never infer success.
            permission_error = error
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            process.poll()
            try:
                os.killpg(process.pid, 0)
            except ProcessLookupError:
                process.wait(timeout=timeout)
                return
            except PermissionError as error:
                permission_error = error
            time.sleep(0.05)
    raise TimeoutError(
        f"BDD server process group {process.pid} did not exit"
    ) from permission_error
