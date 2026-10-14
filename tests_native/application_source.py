"""One retained ordinary mpv process on the authorized private PipeWire core."""
import hashlib
from pathlib import Path
import stat
import subprocess
import wave

import pipewire_inventory as inventory


def generation(pid):
    text = Path('/proc/%d/stat' % pid).read_text()
    if len(text) > 16384:
        raise RuntimeError('application process identity exceeds bound')
    return int(text.rsplit(')', 1)[1].split()[19])


def command(binary, pcm, rate, remote):
    return [str(binary), '--no-config', '--no-video', '--no-terminal',
            '--input-terminal=no', '--ao=pipewire', '--pipewire-remote=' + remote,
            '--audio-device=pipewire/0', '--audio-fallback-to-null=no',
            '--audio-samplerate=' + str(rate), '--audio-channels=stereo',
            '--volume=100', str(pcm)]


def write_wave(path, pcm, rate):
    with wave.open(str(path), 'wb') as output:
        output.setnchannels(2)
        output.setsampwidth(2)
        output.setframerate(rate)
        output.writeframes(pcm)


class RetainedMpv:
    def __init__(self, process, binary, node_name, token):
        self.process = process
        self.binary = Path(binary).resolve(strict=True)
        info = self.binary.stat()
        if not stat.S_ISREG(info.st_mode):
            raise RuntimeError('application binary is not a regular file')
        self.stamp = (info.st_dev, info.st_ino, info.st_size, info.st_mtime_ns)
        self.start = generation(process.pid)
        self.node_name, self.token = node_name, token
        self.binary_sha256 = hashlib.sha256(self.binary.read_bytes()).hexdigest()

    def validate(self, objects):
        if self.process.poll() is not None or generation(self.process.pid) != self.start:
            raise RuntimeError('retained application process exited or changed')
        executable = Path('/proc/%d/exe' % self.process.pid)
        info = executable.stat()
        if executable.resolve(strict=True) != self.binary or (info.st_dev, info.st_ino, info.st_size, info.st_mtime_ns) != self.stamp:
            raise RuntimeError('retained application executable changed')
        snapshot = inventory.observe(objects, 1)
        apps = [a for a in snapshot['applications'] if a['process_pid'] == self.process.pid]
        if len(apps) != 1 or apps[0]['application_names'] != ['mpv'] or len(apps[0]['streams']) != 1:
            raise RuntimeError('ordinary application name/stream identity refused')
        app = apps[0]
        clients = [o for o in objects if o.get('type') == inventory.CLIENT and o['id'] == app['client_id']]
        if len(clients) != 1 or inventory.properties(clients[0]).get('pipewire.sec.pid') not in (self.process.pid, str(self.process.pid)):
            raise RuntimeError('ordinary application native peer PID refused')
        node = next(o for o in objects if o.get('type') == inventory.NODE and o['id'] == app['streams'][0]['node_id'])
        p = inventory.properties(node)
        if (p.get('node.name') != self.node_name or p.get('beacn.test.token') != self.token
                or p.get('node.autoconnect') not in (False, 'false') or 'device.id' in p):
            raise RuntimeError('ordinary application private scope refused')
        return app['application_names'][0]

    def proof(self):
        return {'application': 'mpv', 'binary_sha256': self.binary_sha256,
                'version': subprocess.check_output([str(self.binary), '--version'], text=True, timeout=3).splitlines()[0],
                'process_pid': self.process.pid, 'process_start': self.start,
                'display_name_source': 'observed application.name; no override',
                'audio_backend': 'native PipeWire, explicit private remote; no fallback'}
