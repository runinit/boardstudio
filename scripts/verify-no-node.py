#!/usr/bin/env python3
"""Run a command under strace and report attempts to execute named node/npm."""

import ast
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile

# strace may emit an unfinished syscall; its pathname is still in the first line.
EXEC = re.compile(r'\b(execve|execveat)\((?:[^,]*,\s*)?("(?:[^"\\]|\\.)*")')


def executions(trace):
    found = []
    for line in trace.splitlines():
        if not re.search(r'\bexecve(?:at)?\(', line):
            continue
        match = EXEC.search(line)
        if not match:
            raise ValueError(f"cannot parse exec trace: {line}")
        try:
            path = ast.literal_eval(match.group(2))
        except (SyntaxError, ValueError) as error:
            raise ValueError(f"cannot parse exec pathname: {line}") from error
        if not isinstance(path, str):
            raise ValueError(f"invalid exec pathname: {line}")
        found.append(path)
    return found


def trace_command(strace, trace, command, env=None):
    trace.unlink(missing_ok=True)
    result = subprocess.run(
        [strace, '-f', '-qq', '-s', '65535', '-e', 'trace=execve,execveat',
         '-o', str(trace), '--', *command], env=env, check=False
    )
    return result.returncode, executions(trace.read_text(errors='strict'))


def main(args):
    if args and args[0] == '--':
        args = args[1:]
    if not args:
        print('Usage: python3 scripts/verify-no-node.py -- COMMAND [ARG ...]', file=sys.stderr)
        return 2
    strace = shutil.which('strace')
    if not strace:
        print('Error: strace is required. Install strace and ensure it is on PATH.', file=sys.stderr)
        return 2
    try:
        with tempfile.TemporaryDirectory(prefix='verify-no-node-') as temp:
            directory = Path(temp)
            for name in ('node', 'npm'):
                control = directory / name
                control.write_text('#!/bin/sh\nexit 0\n')
                control.chmod(0o700)
            trace = directory / 'trace'
            env = os.environ.copy()
            env['PATH'] = temp + os.pathsep + env.get('PATH', '')
            # Exercise both PATH lookup and an absolute executable path before trusting tracing.
            status, paths = trace_command(strace, trace, ['/bin/sh', '-c', 'node && "$1"',
                                                          'control', str(directory / 'npm')], env)
            if status != 0 or not all(str(directory / name) in paths for name in ('node', 'npm')):
                print('Error: strace positive control failed; tracing is unavailable or incomplete.', file=sys.stderr)
                return 2
            status, paths = trace_command(strace, trace, args)
            if not paths:
                print('Error: no executable trace recorded; verification is incomplete.', file=sys.stderr)
                return 2
            matches = [path for path in paths if os.path.basename(path) in ('node', 'npm')]
            if matches:
                print('Node/npm execution attempt detected: ' + ', '.join(matches), file=sys.stderr)
                return 1
            if status:
                print(f'Command failed with status {status}; absence verification did not pass.', file=sys.stderr)
                return 1
            print('No named node/npm execution attempts observed.')
            return 0
    except (OSError, UnicodeError, ValueError) as error:
        print(f'Error: unable to verify execution trace: {error}', file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
