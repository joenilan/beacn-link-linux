"""No-USB native sink ownership check inside the existing private-core owner."""
import hashlib
import os
from pathlib import Path
import select
import subprocess
import time

import usb_handshake as owner


def verify(binary, root, arguments, first, second, stop_second, env):
    root.mkdir(mode=0o700, exist_ok=False)
    # Freeze this executable before launching; builds must not replace a running
    # qualification artifact. No shell, desktop core or device argument exists.
    image = Path(binary).read_bytes()
    if len(image) > 64 * 1024 * 1024:
        raise RuntimeError('native playback qualification binary exceeds bound')
    executable = root / 'retained-probe'
    with executable.open('xb') as stream:
        stream.write(image)
    executable.chmod(0o700)
    proof = {'binary_sha256': hashlib.sha256(image).hexdigest(), 'error': None,
             'usb_io_requested': False, 'physical_output_requested': False}
    child = None
    lines = []
    pending = bytearray()
    exited = False
    with (root / 'stderr.log').open('xb') as stderr:
        try:
            child = subprocess.Popen([str(executable), '--authorize-isolated-native-playback',
                                      *arguments[:5], first, second, *arguments[5:]],
                                     env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=stderr)
            end = time.monotonic() + 110
            while time.monotonic() < end:
                if len(pending) + sum(map(len, lines)) > 65536:
                    raise RuntimeError('native playback qualification output exceeded bound')
                while b'\n' in pending:
                    line, _, tail = pending.partition(b'\n')
                    pending[:] = tail
                    text = line.decode('utf8')
                    lines.append(text)
                    if text == 'PLAYBACK EXIT_SOURCE':
                        if exited:
                            raise RuntimeError('native playback source exit requested twice')
                        stop_second()
                        exited = True
                        child.stdin.write(b'EXITED')
                        child.stdin.flush()
                if child.poll() is not None:
                    tail = child.stdout.read(65537)
                    pending.extend(tail)
                    if pending:
                        lines.extend(pending.decode('utf8').splitlines())
                    break
                if select.select([child.stdout], [], [], .05)[0]:
                    pending.extend(os.read(child.stdout.fileno(), 4096))
            else:
                raise RuntimeError('native playback qualification timed out')
            proof['exit'] = child.wait(timeout=2)
            proof['pairs_verified'] = all(any(line.startswith('PLAYBACK pair=%d ' % pair)
                                             and 'sink_release=verified route_release=verified' in line for line in lines)
                                          for pair in range(1, 5))
            proof['faults_verified'] = all('PLAYBACK fault=%d no_retry=true restoration=verified' % stage in lines
                                           for stage in range(1, 5))
            proof['source_exit_requested_once'] = exited
            proof['completed'] = 'PLAYBACK complete pairs=4 faults=4 drop=verified source_exit=verified usb=absent physical=absent' in lines
            if proof['exit'] or not all(proof[k] for k in ('pairs_verified', 'faults_verified', 'source_exit_requested_once', 'completed')):
                raise RuntimeError('native playback qualification failed')
        except Exception as error:
            proof['error'] = type(error).__name__ + ': ' + str(error)
        finally:
            if child is not None:
                if child.poll() is None:
                    child.terminate()
                    try:
                        child.wait(timeout=3)
                    except subprocess.TimeoutExpired:
                        child.kill()
                        child.wait(timeout=2)
                child.stdin.close()
                child.stdout.close()
            proof['owned_child_stopped'] = child is not None and child.poll() is not None
            owner.save(root / 'output.json', lines)
            owner.save(root / 'journal.json', proof)
    if proof['error'] or not proof['owned_child_stopped']:
        raise RuntimeError('native playback owner check failed; inspect retained journal')
    return proof
