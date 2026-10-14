"""Rust start/stop/lifecycle release on an existing owned disposable core."""
import hashlib
import json
import os
from pathlib import Path
import select
import subprocess
import time

import usb_handshake as owner


def verify(binary, root, native_arguments, first, second, stop_second, env, dump):
    root.mkdir(mode=0o700)
    output=[];pending=bytearray();child=None;foreign=None;logs=[]
    proof={'error':None,'binary_sha256':hashlib.sha256(Path(binary).read_bytes()).hexdigest()}
    before=owner.snapshot()
    def send(text):child.stdin.write((text+'\n').encode());child.stdin.flush()
    def until(marker):
        end=time.monotonic()+6
        while time.monotonic()<end:
            while b'\n' in pending:
                line,_,tail=pending.partition(b'\n');pending[:]=tail
                text=line.decode();output.append(text)
                if sum(map(len,output))>65536:raise RuntimeError('control output exceeded bound')
                if marker in text:return text
            if child.poll() is not None:raise RuntimeError('control exited before result')
            if select.select([child.stdout],[],[],.05)[0]:
                data=os.read(child.stdout.fileno(),4096)
                if not data:raise RuntimeError('control output closed')
                pending.extend(data)
        raise RuntimeError('control result timed out: '+marker)
    def stop(process):
        if process is not None and process.poll() is None:
            process.terminate()
            try:process.wait(timeout=2)
            except subprocess.TimeoutExpired:process.kill();process.wait(timeout=2)
    try:
        log=(root/'control.stderr').open('xb');logs.append(log)
        child=subprocess.Popen([str(binary),'--authorize-isolated-routing',*native_arguments[:5],first+'=Media Player',second+'=Other Player',*native_arguments[5:]],
                               env={k:v for k,v in env.items() if not k.startswith('PIPEWIRE_')},stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=log)
        until('phase=Idle')
        send('start');until('phase=Running selected=2 bound=0')
        send('route 1 4');until('phase=Running selected=2 bound=2')
        send('route 0 4');until('CONTROL blocked no-mutation');until('phase=Running selected=2 bound=2')
        proof['opaque_reply_kept_routes']=True
        send('stop');until('phase=Stopped selected=2 bound=0 cleanup=verified released=4 already_absent=0')
        send('stop');until('phase=Stopped selected=2 bound=0 cleanup=verified released=4 already_absent=0')
        proof['explicit_stop_verified']=True
        send('start');until('phase=Running selected=2 bound=0')
        send('route 2 3');until('phase=Running selected=2 bound=2')
        proof['explicit_restart_verified']=True

        # A different retained creator adds one extra link with the same owner
        # tag, outside the Rust executor's receipt/handle ledger. Lifecycle stop
        # must preserve it, despite that tag, and only its creator releases it.
        _,objects=dump();inventory=owner.core.inventory
        app=next(a for a in inventory.observe(objects,1)['applications'] if a['application_id']==first)
        source=next(o for o in objects if o.get('type')==inventory.NODE and o['id']==app['streams'][0]['node_id'])
        token=native_arguments[4]
        if inventory.properties(source).get('beacn.test.token')!=token:raise RuntimeError('foreign probe source scope changed')
        mapping=next(v for v in native_arguments[5:] if v.startswith('4:')).split(':')
        target=next(o for o in objects if o.get('type')==inventory.NODE and o['id']==int(mapping[1]) and int(inventory.properties(o)['object.serial'])==int(mapping[2]))
        if inventory.properties(target).get('beacn.test.token')!=token:raise RuntimeError('foreign probe destination scope changed')
        def port(node,direction):
            return next(o for o in objects if o.get('type')=='PipeWire:Interface:Port' and inventory.properties(o).get('node.id')==node['id'] and inventory.properties(o).get('port.direction')==direction and inventory.properties(o).get('audio.channel')=='FL')
        out=port(source,'out');inp=port(target,'in')
        log=(root/'foreign-creator.log').open('xb');logs.append(log)
        foreign=subprocess.Popen(['pw-cli','-r',native_arguments[1]],env=env,stdin=subprocess.PIPE,stdout=log,stderr=log)
        command='create-link %d %d %d %d ' % (source['id'],int(inventory.properties(out)['port.id']),target['id'],int(inventory.properties(inp)['port.id']))
        foreign.stdin.write((command+json.dumps({'object.linger':False,'beacn.link.owner':token})+'\n').encode());foreign.stdin.flush()
        end=time.monotonic()+3
        while True:
            _,objects=dump()
            links=[o for o in objects if o.get('type')=='PipeWire:Interface:Link' and inventory.properties(o).get('link.output.port')==out['id'] and inventory.properties(o).get('link.input.port')==inp['id']]
            if len(links)==1:break
            if foreign.poll() is not None or time.monotonic()>=end:raise RuntimeError('foreign link did not appear')
            time.sleep(.05)
        stamp=(links[0]['id'],inventory.properties(links[0])['object.serial'])
        owner.save(root/'before-lifecycle.json',objects)
        stop_second()
        end=time.monotonic()+3
        while True:
            _,objects=dump()
            if not any(a['application_id']==second for a in inventory.observe(objects,1)['applications']):break
            if time.monotonic()>=end:raise RuntimeError('departed source remained visible')
            time.sleep(.05)
        send('refresh');until('CONTROL lifecycle-stopped')
        until('phase=Stopped selected=1 bound=0 cleanup=verified released=2 already_absent=2')
        proof['source_exit_stopped_session']=True
        _,objects=dump();owner.save(root/'after-lifecycle.json',objects)
        proof['foreign_same_tag_link_preserved']=any(o.get('type')=='PipeWire:Interface:Link' and (o['id'],inventory.properties(o).get('object.serial'))==stamp for o in objects)
        if not proof['foreign_same_tag_link_preserved']:raise RuntimeError('unowned same-tag link changed')
        send('status');until('phase=Stopped selected=1 bound=0 cleanup=verified released=2 already_absent=2')
        proof['no_automatic_restart']=True
        stop(foreign)
        send('quit');child.wait(timeout=3)
        proof['exit']=child.returncode
    except Exception as error:
        proof['error']=type(error).__name__+': '+str(error)
    finally:
        stop(foreign)
        if child is not None and child.poll() is None:
            try:send('quit');child.wait(timeout=3)
            except (OSError,subprocess.TimeoutExpired):pass
        stop(child)
        for log in logs:log.close()
        proof['desktop_exact_equal']=owner.snapshot()==before
        proof['all_owned_processes_stopped']=all(p is None or p.poll() is not None for p in (child,foreign))
        owner.save(root/'output.json',output);owner.save(root/'journal.json',proof)
    if proof['error'] or proof.get('exit')!=0 or not all(proof.get(k) for k in ['opaque_reply_kept_routes','explicit_stop_verified','explicit_restart_verified','source_exit_stopped_session','foreign_same_tag_link_preserved','no_automatic_restart','desktop_exact_equal','all_owned_processes_stopped']):
        raise RuntimeError('native session control qualification failed; inspect journal')
    return proof
