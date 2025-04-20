import os
import time


PID = None


def get_pid():
    global PID
    if PID is None:
        PID = os.getpid()
    return PID


def log(message):
    print("[PID: {}] {}".format(get_pid(), message))


def work():
    log("Starting work")
    i = 0
    while i < 10_000_000:
        i += 1
    log("Done work")
    return i


def quick_work():
    log("Starting quick work")
    i = 0
    while i < 100_000:
        i += 1
    log("Done quick work")
    return i


def deep_work(n):
    log("Starting deep work")
    if n > 0:
        i = 0
        while i < 10_000 * n:
            i += 1
        return deep_work(n - 1)
    work()


if __name__ == '__main__':
    while True:
        quick_work()
        work()
        deep_work(100)
        time.sleep(0.1)