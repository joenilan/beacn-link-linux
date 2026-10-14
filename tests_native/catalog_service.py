"""Native Rust read-only service qualification on an existing owned private core."""
import hashlib
import json
import os
from pathlib import Path
import select
import subprocess
import time

import application_source
import usb_handshake as owner


def verify(binary, root, runtime, remote, env, dump, first_pid, media, properties, rate):
    root.mkdir(mode=0o700, exist_ok=False)
    _, before = dump()
    baseline = owner.core.filtered(before)
    desktop = owner.snapshot()
    configs = set(Path('/tmp').glob('beacn-linx-catalog-*'))
    clean_env = {k: v for k, v in env.items() if not k.startswith('PIPEWIRE_')}
    pending = bytearray(); output = []
    processes = []; logs = []
    service = None
    proof = {'error': None, 'binary_sha256': hashlib.sha256(Path(binary).read_bytes()).hexdigest()}

    def lines_until(predicate):
        end = time.monotonic() + 6
        while time.monotonic() < end:
            while b'\n' in pending:
                line, _, tail = pending.partition(b'\n'); pending[:] = tail
                text = line.decode('utf8'); output.append(text)
                if sum(map(len, output)) > 65536:raise RuntimeError('service output exceeded bound')
                if predicate(text):return text
            if service.poll() is not None:raise RuntimeError('Rust service exited before acknowledgment')
            ready, _, _ = select.select([service.stdout], [], [], .05)
            if ready:
                data = os.read(service.stdout.fileno(), 4096)
                if not data:raise RuntimeError('Rust service output closed')
                pending.extend(data)
        raise RuntimeError('Rust service acknowledgment timed out')

    def status(count, selected):
        return lines_until(lambda s:s.startswith('STATUS ') and 'applications=%d ' % count in s and 'selected=%d ' % selected in s)

    def send(text):service.stdin.write((text+'\n').encode());service.stdin.flush()
    def find_app(pid):
        line=lines_until(lambda s:s.startswith('APP ') and ' pid=%d ' % pid in s)
        return line.split()[1]
    def launch(label):
        log=(root/(label+'.log')).open('xb');logs.append(log)
        child=subprocess.Popen(application_source.command('/usr/bin/mpv', media, rate, remote),
                               env={**env,'PIPEWIRE_PROPS':json.dumps({**properties,'node.name':'beacn-service-'+label})},
                               stdin=subprocess.DEVNULL,stdout=log,stderr=log)
        processes.append(child);return child
    def stop(child):
        if child.poll() is None:
            child.terminate()
            try:child.wait(timeout=2)
            except subprocess.TimeoutExpired:child.kill();child.wait(timeout=2)

    try:
        log=(root/'service.stderr').open('xb');logs.append(log)
        service=subprocess.Popen([str(binary),'service','30','--private-runtime',str(runtime),'--core',remote],
                                 env=clean_env,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=log)
        processes.append(service)
        status(1,0);first=find_app(first_pid)
        send('select '+first+'=Media Player');status(1,1)
        second=launch('second');status(2,1);second_id=find_app(second.pid)
        send('select '+second_id+'=Other Player');status(2,2)
        stop(second);status(1,1)
        proof['departed_selection_revoked']=True
        reopened=launch('reopened');status(2,1);reopened_id=find_app(reopened.pid)
        if reopened_id==second_id:raise RuntimeError('reopened app reused prior application identity')
        proof['reopened_not_automatically_selected']=True
        send('select '+reopened_id+'=Other Player');status(2,2)
        send('unselect '+first);status(2,1)
        send('quit');service.wait(timeout=3)
        if service.returncode:raise RuntimeError('Rust service failed to exit normally')
        proof['two_app_selections_verified']=True
        proof['service_exit']=service.returncode
        base=[str(binary),'service','1','--private-runtime',str(runtime),'--core',remote]
        bad=subprocess.run(base,input=b'x'*1025+b'\n',env=clean_env,capture_output=True,timeout=4)
        owner.save(root/'bounded-input.json',{'exit':bad.returncode,'stdout':bad.stdout.decode(),'stderr':bad.stderr.decode()})
        proof['bounded_input_rejected']=bad.returncode==2 and b'exceeds bound' in bad.stderr
        eof=subprocess.run(base,input=b'',env=clean_env,capture_output=True,timeout=4)
        proof['eof_stop']=eof.returncode==0 and b'STOP requested' in eof.stdout
        timer=subprocess.Popen(base,env=clean_env,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        processes.append(timer);timer.wait(timeout=4)
        timed_out,timed_err=timer.communicate(timeout=1)
        proof['deadline_stop']=timer.returncode==0 and b'STOP deadline' in timed_out and not timed_err
    except Exception as error:
        proof['error']=type(error).__name__+': '+str(error)
    finally:
        for child in reversed(processes):stop(child)
        if service is not None:
            for stream in (service.stdin,service.stdout):stream.close()
        for log in logs:log.close()
        end=time.monotonic()+3
        while True:
            _, after=dump()
            if owner.core.filtered(after)==baseline or time.monotonic()>=end:break
            time.sleep(.05)
        proof['private_graph_restored']=owner.core.filtered(after)==baseline
        proof['desktop_exact_equal']=owner.snapshot()==desktop
        proof['private_configs_removed']=set(Path('/tmp').glob('beacn-linx-catalog-*'))==configs
        proof['all_owned_processes_stopped']=all(p.poll() is not None for p in processes)
        owner.save(root/'journal.json',proof);owner.save(root/'service-output.json',output)
    if proof['error'] or not all(proof.get(k) for k in ['two_app_selections_verified','departed_selection_revoked','reopened_not_automatically_selected','bounded_input_rejected','eof_stop','deadline_stop','private_graph_restored','desktop_exact_equal','private_configs_removed','all_owned_processes_stopped']):
        raise RuntimeError('Rust catalog/service qualification failed; inspect journal')
    return proof
