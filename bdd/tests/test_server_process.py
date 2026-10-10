import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import unittest

from bdd.features.server_process import start_server_process, stop_server_process


class ServerProcessTests(unittest.TestCase):
    def start_writer(self, directory, ignore_term=False):
        source = """
import os, signal, sys, time
from pathlib import Path
if os.fork():
    signal.signal(signal.SIGTERM, lambda *_: sys.exit(0))
    while True:
        signal.pause()
def finish(*_):
    time.sleep(0.15)
    Path("finished").write_text("child exited")
    sys.exit(0)
signal.signal(signal.SIGTERM, signal.SIG_IGN if sys.argv[1] == "ignore" else finish)
print("ready", flush=True)
while True:
    signal.pause()
"""
        process = start_server_process(
            [sys.executable, "-u", "-c", source, "ignore" if ignore_term else "exit"],
            cwd=directory,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        self.addCleanup(process.stdout.close)
        self.addCleanup(stop_server_process, process)
        self.assertEqual(process.stdout.readline().strip(), "ready")
        return process

    def test_waits_for_child_after_launcher_exits(self):
        with tempfile.TemporaryDirectory() as directory:
            process = self.start_writer(directory)
            stop_server_process(process)
            self.assertEqual(Path(directory, "finished").read_text(), "child exited")
            with self.assertRaises(ProcessLookupError):
                os.killpg(process.pid, 0)

    def test_kills_child_that_ignores_termination(self):
        with tempfile.TemporaryDirectory() as directory:
            process = self.start_writer(directory, ignore_term=True)
            stop_server_process(process, timeout=0.3)
            with self.assertRaises(ProcessLookupError):
                os.killpg(process.pid, 0)

    def test_already_exited_process_is_reaped(self):
        process = start_server_process([sys.executable, "-c", "pass"])
        process.wait(timeout=5)
        stop_server_process(process)
        self.assertEqual(process.returncode, 0)
