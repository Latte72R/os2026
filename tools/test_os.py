#!/usr/bin/env python3

import os
import select
import subprocess
import sys
import time


class BeigecoreTest:
    def __init__(self, simulator: str, rom: str, ram: str) -> None:
        self.output = bytearray()
        self.process = subprocess.Popen(
            [simulator, rom, ram],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
        )

    def read_more(self, timeout: float) -> None:
        assert self.process.stdout is not None
        ready, _, _ = select.select([self.process.stdout], [], [], timeout)
        if not ready:
            return

        chunk = os.read(self.process.stdout.fileno(), 4096)
        if not chunk:
            raise RuntimeError(f"simulator exited unexpectedly with {self.process.poll()}")
        self.output.extend(chunk)

    def read_until(self, expected: bytes, start: int = 0, timeout: float = 90.0) -> None:
        deadline = time.monotonic() + timeout
        while expected not in self.output[start:]:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError(f"did not receive {expected!r}")
            self.read_more(remaining)

    def command(self, command: str, expected: bytes) -> None:
        self.send(command.encode("ascii") + b"\r", expected)

    def send(self, data: bytes, expected: bytes) -> None:
        assert self.process.stdin is not None
        start = len(self.output)
        self.process.stdin.write(data)
        self.process.stdin.flush()
        self.read_until(expected, start=start)

    def shutdown(self) -> None:
        assert self.process.stdin is not None
        self.process.stdin.write(b"poweroff\r")
        self.process.stdin.flush()
        return_code = self.process.wait(timeout=30.0)
        assert self.process.stdout is not None
        self.output.extend(self.process.stdout.read())
        if return_code != 0:
            raise RuntimeError(f"simulator exited with {return_code}")

    def close(self) -> None:
        if self.process.poll() is None:
            self.process.terminate()
            try:
                self.process.wait(timeout=2.0)
            except subprocess.TimeoutExpired:
                self.process.kill()


def main() -> int:
    if len(sys.argv) != 5:
        print(f"usage: {sys.argv[0]} SIMULATOR ROM RAM LOG", file=sys.stderr)
        return 2

    simulator, rom, ram, log = sys.argv[1:]
    test = BeigecoreTest(simulator, rom, ram)

    try:
        test.read_until(b"vertos> ")
        test.send(b"echo cancelled\x03", b"^C\r\r\nvertos> ")
        test.command("help", b"arrow keys edit input and history")
        test.send(
            b"echo hllo" + b"\x1b[D" * 4 + b"\x1b[C" + b"e\r",
            b"\r\r\nhello\r\r\nvertos> ",
        )
        test.send(b"\x1b[A\r", b"\r\r\nhello\r\r\nvertos> ")
        test.send(
            b"\x1b[A\x1b[Becho arrows\r",
            b"\r\r\narrows\r\r\nvertos> ",
        )
        test.command("ps", b"1    0    runnable")
        test.command("kill", b"usage: kill [-9|-KILL|-STOP|-CONT] <pid>")
        test.command("ps &", b"cannot run shell built-in in background")
        test.command("workers", b"[3] exited 2\r\r\nvertos> ")
        if b"[worker 1] step 0\r\r\n[worker 2] step 0" not in test.output:
            raise RuntimeError("workers did not run round-robin")
        if b"[worker 1] step 9\r\r\n[worker 2] step 9" not in test.output:
            raise RuntimeError("workers did not complete ten steps")
        test.command("yes", b"[4] running yes")
        test.send(b"\x1a", b"[4] stopped\r\r\nvertos> ")
        test.command("jobs", b"[4] stopped")
        test.command("bg", b"[4] running")
        test.command("fg", b"[4] foreground")
        test.send(b"\x03", b"[4] interrupted\r\r\nvertos> ")
        test.command("jobs", b"no jobs")
        test.command("yes", b"[5] running yes")
        test.send(b"\x1a", b"[5] stopped\r\r\nvertos> ")
        test.command("kill 5", b"[5] terminated")
        test.command("yes &", b"[6] running yes")
        test.command("kill -STOP 6", b"[6] stopped")
        test.command("kill -9 6", b"[6] killed")
        test.command("yes &", b"[7] running yes")
        test.command("kill -STOP 7", b"[7] stopped")
        test.command("kill -CONT 7", b"[7] running")
        test.command("kill -KILL 7", b"[7] killed")
        test.command("workers &", b"[8,9] running workers")
        test.command("wait", b"[9] exited 2\r\r\nvertos> ")
        test.command("echo beigecore", b"beigecore\r\r\nvertos> ")
        test.shutdown()
        if b"system poweroff" not in test.output:
            raise RuntimeError("system controller did not report poweroff")
    except Exception as error:
        print(test.output.decode("utf-8", errors="replace"), file=sys.stderr)
        print(f"FAIL: {error}", file=sys.stderr)
        return 1
    finally:
        test.close()
        with open(log, "wb") as log_file:
            log_file.write(test.output)

    print("PASS: vertos U-mode shell and job control on beigecore")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
