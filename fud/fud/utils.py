import sys
import logging as log
import shutil
from tempfile import TemporaryDirectory, NamedTemporaryFile, TemporaryFile
from io import BytesIO
from pathlib import Path
import subprocess
import os

from . import errors


def eprint(*args, **kwargs):
    print(*args, **kwargs, file=sys.stderr)


def is_warning():
    return log.getLogger().level <= log.WARNING


def is_info():
    return log.getLogger().level <= log.INFO


def is_debug():
    return log.getLogger().level <= log.DEBUG


def unwrap_or(val, default):
    if val is not None:
        return val

    return default


def logging_setup(args):
    # Color for warning, error, and info messages.
    log.addLevelName(log.INFO, "\033[1;34m%s\033[1;0m" % log.getLevelName(log.INFO))
    log.addLevelName(
        log.WARNING, "\033[1;33m%s\033[1;0m" % log.getLevelName(log.WARNING)
    )
    log.addLevelName(log.ERROR, "\033[1;31m%s\033[1;0m" % log.getLevelName(log.ERROR))

    # Set verbosity level.
    level = None
    if "verbose" not in args or args.verbose == 0:
        level = log.WARNING
    elif args.verbose == 1:
        level = log.INFO
    elif args.verbose >= 2:
        level = log.DEBUG

    log.basicConfig(
        format="[fud] %(levelname)s: %(message)s", stream=sys.stderr, level=level
    )

    try:
        import paramiko

        paramiko.util.logging.getLogger().setLevel(level)
    except ModuleNotFoundError:
        pass


class Directory:
    def __init__(self, name):
        self.name = name

    def remove(self):
        shutil.rmtree(self.name)


class TmpDir(Directory):
    def __init__(self):
        self.tmpdir_obj = TemporaryDirectory()
        self.name = self.tmpdir_obj.name

    def remove(self):
        self.tmpdir_obj.cleanup()

    def __str__(self):
        return self.name


class Conversions:
    @staticmethod
    def path_to_directory(data):
        if data.is_dir():
            return Directory(data.name)
        else:
            raise errors.SourceConversionNotDirectory(data.name)

    @staticmethod
    def path_to_stream(data):
        return open(data, "rb")

    @staticmethod
    def stream_to_path(data):
        with NamedTemporaryFile("wb", delete=False) as tmpfile:
            tmpfile.write(data.read())
            return Path(tmpfile.name)

    @staticmethod
    def stream_to_bytes(data):
        return data.read()

    @staticmethod
    def bytes_to_stream(data):
        return BytesIO(data)

    @staticmethod
    def bytes_to_string(data):
        return data.decode("UTF-8")

    @staticmethod
    def string_to_bytes(data):
        return data.encode("UTF-8")


class SpinnerWrapper:
    """
    Wraps a spinner object.
    """

    def __init__(self, spinner, save):
        self.spinner = spinner
        self.save = save
        self.stage_text = ""
        self.step_text = ""

    def _update(self):
        if self.step_text != "":
            self.spinner.start(f"{self.stage_text}: {self.step_text}")
        else:
            self.spinner.start(f"{self.stage_text}")

    def start_stage(self, text):
        self.stage_text = text
        self._update()

    def end_stage(self):
        if self.save:
            self.spinner.succeed()

    def start_step(self, text):
        self.step_text = text
        self._update()

    def end_step(self):
        if self.save:
            self.spinner.succeed()
        self.step_text = ""
        self._update()

    def succeed(self):
        self.spinner.succeed()

    def fail(self, text=None):
        self.spinner.fail(text)

    def stop(self):
        self.spinner.stop()


def shell(cmd, stdin=None, stdout_as_debug=False):
    """
    Runs `cmd` in the shell and returns a stream of the output.
    Raises `errors.StepFailure` if the command fails.
    """

    if isinstance(cmd, list):
        cmd = " ".join(cmd)

    if stdout_as_debug:
        cmd += ">&2"

    assert isinstance(cmd, str)

    log.debug(cmd)

    stdout = TemporaryFile()
    stderr = None
    # if we are not in debug mode, capture stderr
    if not is_debug():
        stderr = TemporaryFile()

    proc = subprocess.Popen(
        cmd, shell=True, stdin=stdin, stdout=stdout, stderr=stderr, env=os.environ
    )
    proc.wait()
    stdout.seek(0)
    if proc.returncode != 0:
        if stderr is not None:
            stderr.seek(0)
            raise errors.StepFailure(
                cmd, stdout.read().decode("UTF-8"), stderr.read().decode("UTF-8")
            )
        else:
            raise errors.StepFailure(cmd, "No stdout captured.", "No stderr captured.")
    return stdout


def transparent_shell(cmd):
    """
    Runs `cmd` in the shell. Does not capture output or input. Does nothing
    fancy and returns nothing
    """
    if isinstance(cmd, list):
        cmd = " ".join(cmd)

    assert isinstance(cmd, str)

    log.debug(cmd)

    proc = subprocess.Popen(cmd, env=os.environ, shell=True)

    proc.wait()


def parse_profiling_input(args):
    """
    Returns a mapping from stage to steps from the `profiled_stages` argument.
    For example, if the user passes in `-pr a.a1 a.a2 b.b1 c`, this will return:
    {"a" : ["a1", "a2"], "b" : ["b1"], "c" : [] }
    """
    stages = {}
    if args.profiled_stages is None:
        return stages
    # Retrieve all stages.
    for stage in args.profiled_stages:
        if "." not in stage:
            stages[stage] = []
        else:
            s, _ = stage.split(".")
            stages[s] = []
    # Append all steps.
    for stage in args.profiled_stages:
        if "." not in stage:
            continue
        _, step = stage.split(".")
        stages[s].append(step)
    return stages


def profiling_dump(stage, phases, durations):
    """
    Returns time elapsed during each stage or step of the fud execution.
    """
    assert all(hasattr(p, "name") for p in phases), "expected to have name attribute."

    def name_and_space(s: str) -> str:
        # Return a string containing `s` followed by max(32 - len(s), 1) spaces.
        return "".join((s, max(32 - len(s), 1) * " "))

    return f"{name_and_space(stage)}elapsed time (s)\n" + "\n".join(
        f"{name_and_space(p.name)}{round(t, 3)}" for p, t in zip(phases, durations)
    )


def profiling_csv(stage, phases, durations):
    """
    Dumps the profiling information into a CSV format.
    For example, with
        stage:     `x`
        phases:    ['a', 'b', 'c']
        durations: [1.42, 2.0, 3.4445]
    The output will be:
    ```
    x,a,1.42
    x,b,2.0
    x,c,3.444
    ```
    """
    assert all(hasattr(p, "name") for p in phases), "expected to have name attribute."
    return "\n".join(
        [f"{stage},{p.name},{round(t, 3)}" for (p, t) in zip(phases, durations)]
    )


def profile_stages(stage, phases, durations, is_csv):
    """
    Returns either a human-readable or CSV format profiling information,
    depending on `is_csv`.
    """
    kwargs = {
        "stage": stage,
        "phases": phases,
        "durations": durations,
    }
    return profiling_csv(**kwargs) if is_csv else profiling_dump(**kwargs)
