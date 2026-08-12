#!/usr/bin/env python3

import os
import select
import subprocess
import sys
import time


class ShellTest:
    def __init__(self, kernel: str) -> None:
        self.output = bytearray()
        self.process = subprocess.Popen(
            ["./scripts/run.sh", kernel],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
        )

    def read_until(self, expected: bytes, timeout: float = 10.0) -> None:
        assert self.process.stdout is not None
        deadline = time.monotonic() + timeout

        while expected not in self.output:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError(f"did not receive {expected!r}")

            ready, _, _ = select.select([self.process.stdout], [], [], remaining)
            if not ready:
                continue

            chunk = os.read(self.process.stdout.fileno(), 4096)
            if not chunk:
                raise RuntimeError(
                    f"QEMU exited with {self.process.poll()} before {expected!r}"
                )
            self.output.extend(chunk)

    def command(self, command: str, expected: bytes) -> None:
        self.send(command.encode("ascii") + b"\r", expected)

    def send(self, data: bytes, expected: bytes) -> None:
        assert self.process.stdin is not None
        start = len(self.output)
        self.process.stdin.write(data)
        self.process.stdin.flush()

        deadline = time.monotonic() + 10.0
        while expected not in self.output[start:]:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError(f"input {data!r} did not produce {expected!r}")
            self.read_more(remaining)

    def read_more(self, timeout: float) -> None:
        assert self.process.stdout is not None
        ready, _, _ = select.select([self.process.stdout], [], [], timeout)
        if not ready:
            return
        chunk = os.read(self.process.stdout.fileno(), 4096)
        if not chunk:
            raise RuntimeError(f"QEMU exited unexpectedly with {self.process.poll()}")
        self.output.extend(chunk)

    def finish(self) -> None:
        assert self.process.stdin is not None
        self.process.stdin.write(b"poweroff\r")
        self.process.stdin.flush()
        return_code = self.process.wait(timeout=10.0)
        if return_code != 0:
            raise RuntimeError(f"QEMU exited with {return_code}")

    def close(self) -> None:
        if self.process.poll() is None:
            self.process.terminate()
            try:
                self.process.wait(timeout=2.0)
            except subprocess.TimeoutExpired:
                self.process.kill()


def main() -> int:
    if len(sys.argv) != 2:
        print(f"usage: {sys.argv[0]} KERNEL", file=sys.stderr)
        return 2

    test = ShellTest(sys.argv[1])
    try:
        test.read_until(b"vertos> ")
        test.command("help", b"Ctrl-C interrupts and Ctrl-Z stops")
        test.command("ps", b"1    0    runnable")
        test.command("demo", b"[2] exited 1\r\r\nvertos> ")
        test.command("yes", b"[3] running yes")
        test.send(b"\x1a", b"[3] stopped\r\r\nvertos> ")
        test.command("jobs", b"[3] stopped")
        test.command("fg 3", b"y\r\r\n")
        test.send(b"\x03", b"[3] interrupted\r\r\nvertos> ")
        test.command("jobs", b"no jobs")
        test.command("yes", b"[4] running yes")
        test.send(b"\x1a", b"[4] stopped\r\r\nvertos> ")
        test.command("kill 4", b"[4] terminated")
        test.command("demo &", b"[5] running demo")
        test.command("wait 5", b"[5] exited 1")
        test.command("echo hello", b"echo hello\r\r\nhello\r\r\nvertos> ")
        test.finish()
    except Exception as error:
        print(test.output.decode("utf-8", errors="replace"), file=sys.stderr)
        print(f"FAIL: {error}", file=sys.stderr)
        return 1
    finally:
        test.close()

    print("PASS: U-mode shell, job control, and cooperative processes")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
