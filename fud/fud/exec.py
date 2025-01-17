import logging as log
import shutil
import sys
import time
from pathlib import Path

from halo import Halo

from . import errors, utils
from .stages import Source, SourceType


def discover_implied_stage(filename, config, possible_dests=None):
    """
    Use the mapping from filename extensions to stages to figure out which
    stage was implied.
    """
    if filename is None:
        raise errors.NoInputFile(possible_dests)

    suffix = Path(filename).suffix
    for (name, stage) in config["stages"].items():
        if "file_extensions" in stage:
            for ext in stage["file_extensions"]:
                if suffix == ext:
                    return name

    # no stages corresponding with this file extension where found
    raise errors.UnknownExtension(filename)


def construct_path(args, config, through):
    """
    Construct the path of stages implied by the passed arguments.
    """
    # find source
    source = args.source
    if source is None:
        source = discover_implied_stage(args.input_file, config)

    # find target
    target = args.dest
    if target is None:
        target = discover_implied_stage(args.output_file, config)

    path = config.REGISTRY.make_path(source, target, through)
    if path is None:
        raise errors.NoPathFound(source, target, through)

    # If the path doesn't execute anything, it is probably an error.
    if len(path) == 0:
        raise errors.TrivialPath(source)

    return path


def run_fud(args, config):
    """
    Execute all the stages implied by the passed `args`
    """
    # check if input_file exists
    input_file = None
    if args.input_file is not None:
        input_file = Path(args.input_file)
        if not input_file.exists():
            raise FileNotFoundError(input_file)

    path = construct_path(args, config, args.through)

    # check if we need `-o` specified
    if path[-1].output_type == SourceType.Directory and args.output_file is None:
        raise errors.NeedOutputSpecified(path[-1])

    # if we are doing a dry run, print out stages and exit
    if args.dry_run:
        print("fud will perform the following steps:")
        for ed in path:
            print(f"Stage: {ed.name}")
            ed.dry_run()
        return

    # spinner is disabled if we are in debug mode, doing a dry_run, or are in quiet mode
    spinner_enabled = not (utils.is_debug() or args.dry_run or args.quiet)

    # Execute the path transformation specification.
    with Halo(
        spinner="dots", color="cyan", stream=sys.stderr, enabled=spinner_enabled
    ) as sp:

        sp = utils.SpinnerWrapper(sp, save=log.getLogger().level <= log.INFO)

        # construct a source object for the input
        data = None
        if input_file is None:
            data = Source(None, SourceType.UnTyped)
        else:
            data = Source(Path(str(input_file)), SourceType.Path)

        profiled_stages = utils.parse_profiling_input(args)
        # tracks profiling information requested by the flag (if set).
        collected_for_profiling = {}
        # tracks the approximate time elapsed to run each stage.
        overall_durations = []

        # run all the stages
        for ed in path:
            txt = f"{ed.src_stage} → {ed.target_stage}" + (
                f" ({ed.name})" if ed.name != ed.src_stage else ""
            )
            sp.start_stage(txt)
            try:
                if ed._no_spinner:
                    sp.stop()
                begin = time.time()
                data = ed.run(data, sp=sp if ed._no_spinner else None)
                overall_durations.append(time.time() - begin)
                sp.end_stage()
            except errors.StepFailure as e:
                sp.fail()
                print(e)
                exit(-1)
            # Collect profiling information for this stage.
            if ed.name in profiled_stages:
                collected_for_profiling[ed.name] = ed
        sp.stop()

        if args.profiled_stages is not None:
            if args.profiled_stages == []:
                # No stages provided; collect overall stage durations.
                data.data = utils.profile_stages(
                    "stage", [ed for ed in path], overall_durations, args.csv
                )
            else:
                # Otherwise, gather profiling data for each stage and steps provided.
                def gather_profiling_data(stage, steps):
                    data = collected_for_profiling.get(stage)
                    # Verify this is a valid stage.
                    if data is None:
                        raise errors.UndefinedStage(stage)
                    # Verify the steps are valid.
                    valid_steps = [s.name for s in data.steps]
                    invalid_steps = [s for s in steps if s not in valid_steps]
                    if invalid_steps:
                        raise errors.UndefinedSteps(stage, invalid_steps)
                    # If no specific steps provided for this stage, append all of them.
                    profiled_steps = [
                        s for s in data.steps if steps == [] or s.name in steps
                    ]
                    # Gather all the step names that are being profiled.
                    profiled_names = [s.name for s in profiled_steps]
                    profiled_durations = [data.durations[s] for s in profiled_names]
                    return utils.profile_stages(
                        stage,
                        profiled_steps,
                        profiled_durations,
                        args.csv,
                    )

                data.data = "\n".join(
                    gather_profiling_data(stage, steps)
                    for stage, steps in profiled_stages.items()
                )

        # output the data or profiling information.
        if args.output_file is not None:
            if data.typ == SourceType.Directory:
                shutil.move(data.data.name, args.output_file)
            else:
                with Path(args.output_file).open("wb") as f:
                    f.write(data.convert_to(SourceType.Bytes).data)
        elif data:
            print(data.convert_to(SourceType.String).data)
