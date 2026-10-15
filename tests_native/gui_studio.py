"""Bounded native GUI + owned physical bridge qualification, no desktop takeover."""
import os
from pathlib import Path
import selectors
import signal
import subprocess
import time


def validate(stage, application, physical, continuous, cycles, catalog, source_exit):
    if stage != 'run' or not application or not physical or not continuous or cycles != 40 or catalog or source_exit:
        raise RuntimeError('Studio GUI requires an explicit 40-cycle ordinary-app physical run')


def physical_request(raw, pair, observed, cycles, started, active):
    starting=raw==f'PHYSICAL START {pair}'.encode()
    stopping=raw==f'PHYSICAL STOP {pair}'.encode()
    if not ((starting and not started and observed>=1 and observed+8<cycles) or (stopping and active)):
        raise RuntimeError('Studio GUI physical sequence refused')
    return starting


class NativeChild:
    """Pin a reported GUI only after proving it is our wrapper's descendant."""
    def __init__(self,pid,wrapper,binary):
        self.pid=pid;self.wrapper=wrapper
        process=Path('/proc')/str(pid)
        self.generation=process.joinpath('stat').read_text().rsplit(')',1)[1].split()[19]
        if process.stat().st_uid!=os.getuid() or not process.joinpath('exe').samefile(binary):
            raise RuntimeError('native GUI executable/user refused')
        ancestor=pid
        for _ in range(8):
            status=(Path('/proc')/str(ancestor)/'status').read_text()
            ancestor=int(next(line.split()[1] for line in status.splitlines() if line.startswith('PPid:')))
            if ancestor==wrapper.pid:break
            if ancestor<=1:raise RuntimeError('native GUI is not owned by this wrapper')
        else:raise RuntimeError('native GUI ancestry exceeds bound')
        self.pidfd=os.pidfd_open(pid)
        if self.poll() is not None:
            os.close(self.pidfd)
            raise RuntimeError('native GUI changed while pinning its process')
    def poll(self):
        try:
            fields=(Path('/proc')/str(self.pid)/'stat').read_text().rsplit(')',1)[1].split()
            return None if fields[19]==self.generation and fields[0]!='Z' else -1
        except OSError:return -1
    def close(self):
        try:
            for sig,seconds in [(signal.SIGTERM,3),(signal.SIGKILL,2)]:
                if self.poll() is not None:break
                try:signal.pidfd_send_signal(self.pidfd,sig)
                except ProcessLookupError:break
                end=time.monotonic()+seconds
                while self.poll() is None and time.monotonic()<end:time.sleep(.05)
            return self.poll() is not None
        finally:os.close(self.pidfd)


def execute(binary, root, selected, cache, app_id, pair, cycles, native_arguments,
            fresh, concurrent, pulse):
    """Own Xvfb/bus + direct Rust child PID; exact bounded physical IPC only."""
    root.mkdir(mode=0o700)
    owned = []
    logs = []
    child = None
    native = None
    active = False
    started = False
    observed = 0
    verified_routes = 0
    pending = bytearray()
    output = {'stdout': bytearray(), 'stderr': bytearray()}
    result = {'exit': 2, 'child_exited': True, 'timed_out': False,
              'physical_requests': 0, 'inventory_requests': 0, 'studio_gui': True}
    deadline = time.monotonic() + 90

    def log(name):
        stream = (root / name).open('xb'); logs.append(stream); return stream

    def stop(process):
        if process is None or process.poll() is not None:
            return
        process.terminate()
        try:
            process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            process.kill(); process.wait(timeout=3)

    try:
        env = {k: v for k, v in os.environ.items() if not k.startswith('PIPEWIRE_') and k not in ('WAYLAND_DISPLAY', 'XDG_SESSION_TYPE', 'DBUS_SESSION_BUS_ADDRESS')}
        # Pin the test process to the installed CPU Vulkan implementation. GPU
        # enumeration can stall on this live laptop's hybrid graphics stack.
        software_icd=Path('/usr/share/vulkan/icd.d/lvp_icd.json')
        if not software_icd.is_file():raise RuntimeError('isolated GUI software renderer unavailable')
        env.update(LIBGL_ALWAYS_SOFTWARE='1', WINIT_UNIX_BACKEND='x11', LINX_STUDIO_GUI_TEST='1', LIBUSB_DEBUG='0',
                   VK_DRIVER_FILES=str(software_icd),VK_ICD_FILENAMES=str(software_icd))
        command = ['xvfb-run','-a','-s','-screen 0 1280x900x24','dbus-run-session','--',str(binary), '--authorize-bounded-studio-gui', str(root/'gui'),str(selected),str(cache),
                   app_id,str(pair),str(cycles),*native_arguments]
        child = subprocess.Popen(command,env=env,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        owned.append(child);result['private_child_pid']=child.pid
        native = None
        stream_logs={name:log('gui.'+name) for name in output}
        os.set_blocking(child.stdin.fileno(), False)
        with selectors.DefaultSelector() as selector:
            for name in output:
                pipe=getattr(child,name);os.set_blocking(pipe.fileno(),False);selector.register(pipe,selectors.EVENT_READ,name)
            last_guard=0
            while selector.get_map():
                now=time.monotonic()
                if now>=deadline:result['timed_out']=True;raise RuntimeError('Studio GUI deadline exceeded')
                pulse()
                if now-last_guard>=.5:
                    fresh(result['inventory_requests']);result['inventory_requests']+=1;last_guard=now
                for key,_ in selector.select(.1):
                    data=os.read(key.fileobj.fileno(),4096)
                    if not data:selector.unregister(key.fileobj);continue
                    output[key.data].extend(data)
                    stream_logs[key.data].write(data);stream_logs[key.data].flush()
                    if len(output[key.data])>65536:raise RuntimeError('Studio GUI output exceeds bound')
                    if key.data!='stdout':continue
                    pending.extend(data)
                    while b'\n' in pending:
                        raw,_,tail=pending.partition(b'\n');pending=bytearray(tail)
                        if raw.startswith(b'STUDIO OWNER '):
                            if native is not None:raise RuntimeError('duplicate native GUI owner')
                            native=NativeChild(int(raw.decode('ascii').split()[2]),child,binary)
                        elif raw.startswith(b'STUDIO CYCLE '):
                            fields=raw.decode('ascii').split()
                            count=int(fields[2])
                            if count!=observed+1 or count>cycles:raise RuntimeError('Studio GUI cycle sequence refused')
                            observed=count
                            assignment=int(fields[3].removeprefix('assigned='))
                            if assignment not in range(5):raise RuntimeError('invalid verified assignment')
                            if assignment:verified_routes+=1
                        elif raw.startswith(b'PHYSICAL'):
                            starting=physical_request(raw,pair,observed,cycles,started,active)
                            if native is None or concurrent('start' if starting else 'stop',native) is not True:raise RuntimeError('physical state unconfirmed')
                            active=starting;started|=starting
                            result['physical_requests']+=int(starting)
                            message=f'PHYSICAL {"ACTIVE" if starting else "COMPLETE"} {pair}\n'.encode()
                            if os.write(child.stdin.fileno(),message)!=len(message):raise RuntimeError('partial GUI physical delivery')
                            if not starting:result['physical_stage_completed']=True
                    if len(pending)>4096:raise RuntimeError('Studio GUI line exceeds bound')
            child.wait(timeout=max(.01,deadline-time.monotonic()))
        evidence=root/'gui/studio-result.txt'
        result['gui_hardware_verified']=child.returncode==0 and observed==cycles and evidence.is_file() and evidence.read_text()==f'hardware_session=true cycles={cycles} route_release=verified transport_release=verified\n' and not (root/'gui/failure.txt').exists()
        if not result['gui_hardware_verified']:raise RuntimeError('Studio GUI session proof failed')
    except (OSError,ValueError,RuntimeError,subprocess.TimeoutExpired) as error:
        result['supervisor_error']=type(error).__name__+': '+str(error)
        if child is not None:
            try:concurrent('abort',native if native is not None else child)
            except (OSError,ValueError,RuntimeError):pass
            child.stdin.close()
            try:child.wait(timeout=4)
            except subprocess.TimeoutExpired:pass
    finally:
        native_stopped=native.close() if native is not None else True
        for process in reversed(owned):stop(process)
        for stream in logs:stream.close()
        if child is not None:
            result.update(exit=child.returncode,child_exited=child.poll() is not None and native_stopped)
            for pipe in (child.stdin,child.stdout,child.stderr):pipe.close()
        result.update({name:data.decode('utf-8','replace') for name,data in output.items()})
        result['gui_surface_processes_stopped']=native_stopped and all(p.poll() is not None for p in owned)
        result['private_routes_verified_count']=verified_routes
        result['routes_applied']=verified_routes>0
    return result
