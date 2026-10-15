"""Metadata-only witness for the Rust-owned Studio adapter. Never creates audio."""
from pathlib import Path
import time
import studio_pipewire_bridge as bridge


def reference(text):
    fields=text.split(':')
    if len(fields)!=2 or not all(x.isascii() and x.isdecimal() for x in fields):
        raise RuntimeError('native playback reference syntax refused')
    result=tuple(map(int,fields))
    if not 0<result[0]<2**32 or not 0<result[1]<2**64:
        raise RuntimeError('native playback reference outside bounds')
    return result


def identity(obj):
    return obj['id'],int(bridge.props(obj).get('object.serial',0))


class Observer:
    def __init__(self,selected,before,root,dump,server,destinations,source,target,token,pair):
        self.selected=selected;self.before=before;self.root=root;root.mkdir(mode=0o700)
        self.dump=dump;self.server=server;self.destinations=destinations;self.source=source
        self.target=target;self.token=token;self.pair=pair
        self.native=None;self.node=None;self.links=set();self.active=False;self.started=False;self.complete=False
        self.ports=set()
        self.observations=[];self.closed=False;self.last_sample=0
        self.last_advance=None
        self.journal={'physical_owner':'native Rust/C','audio_payload_reads':False,'error':None}

    def owned_links(self,objects,native):
        """Validate auxiliary metadata before filtering a source-only projection.

        Creation/removal can precede the marker being read. Exact native client,
        selected monitor parent, ALSA path and owner must still match. The full
        independent active graph is checked after the acknowledged start marker.
        """
        nodes={o['id']:o for o in objects if o.get('type')==bridge.NODE}
        sources={o['id'] for o in objects if o.get('type')==bridge.NODE
                 and bridge.props(o).get('media.class')=='Stream/Output/Audio'
                 and bridge.client_pid(objects,o)==self.source.pid}
        auxiliaries=[]
        for link in (o for o in objects if o.get('type')==bridge.LINK):
            p=bridge.props(link)
            if int(p['link.output.node']) in sources:continue
            hardware=nodes.get(int(p['link.input.node']))
            if (native is None or native.poll() is not None or hardware is None
                    or bridge.client_pid(objects,link)!=native.pid
                    or bridge.client_pid(objects,hardware)!=native.pid
                    or p.get('beacn.link.owner')!=self.token
                    or int(p['link.output.node'])!=self.target['id']
                    or bridge.props(hardware).get('beacn.link.owner')!=self.token
                    or bridge.props(hardware).get('api.alsa.path')!='hw:%d,0'%self.before['card']):
                raise RuntimeError('unexpected auxiliary link in native playback graph')
            auxiliaries.append(link)
        if len(auxiliaries)>2:raise RuntimeError('native playback auxiliary link count refused')
        return auxiliaries

    def filtered(self,objects,native):
        auxiliary=self.owned_links(objects,native)
        return [o for o in objects if o not in auxiliary]

    def graph(self,objects):
        auxiliary=self.owned_links(objects,self.native)
        if {identity(o) for o in auxiliary}!=self.links:
            raise RuntimeError('native playback acknowledged link identities changed')
        hardware=[o for o in objects if o.get('type')==bridge.NODE and identity(o)==self.node]
        if len(hardware)!=1:raise RuntimeError('native playback acknowledged node changed')
        ports={o['id']:o for o in objects if o.get('type')==bridge.PORT}
        inputs=[o for o in ports.values() if bridge.props(o).get('node.id')==hardware[0]['id']]
        if (len(inputs)!=8 or any(bridge.props(o).get('port.direction')!='in' or not identity(o)[1] for o in inputs)
                or sorted(bridge.props(o).get('audio.channel') for o in inputs)!=['AUX%d'%i for i in range(8)]):
            raise RuntimeError('native playback eight-channel layout changed')
        ports_now={identity(o) for o in inputs}
        if self.ports and self.ports!=ports_now:raise RuntimeError('native playback port generations changed')
        self.ports=ports_now
        expected=set(bridge.pair_channels(self.pair));actual=set()
        for link in auxiliary:
            p=bridge.props(link);out=ports[int(p['link.output.port'])];input_=ports[int(p['link.input.port'])]
            if (link.get('info',{}).get('state')!='active' or bridge.props(out).get('port.monitor') is not True
                    or bridge.props(out).get('port.direction')!='out' or bridge.props(out).get('node.id')!=self.target['id']
                    or bridge.props(input_).get('port.direction')!='in' or bridge.props(input_).get('node.id')!=hardware[0]['id']):
                raise RuntimeError('native playback monitor link is inactive or non-monitor')
            actual.add((bridge.props(out).get('audio.channel'),bridge.props(input_).get('audio.channel')))
        if actual!=expected:raise RuntimeError('native playback stereo-to-AUX mapping changed')
        bridge.assignment_one([o for o in objects if o not in auxiliary],self.target,self.destinations.pid,
                              self.source.pid,self.native.pid,self.token)

    def event(self,raw,native):
        fields=raw.decode('ascii').split()
        if fields[:3]==['NATIVE','PLAYBACK','ACTIVE'] and len(fields)==6:
            if (self.started or fields[3]!=str(self.pair) or not fields[4].startswith('node=')
                    or not fields[5].startswith('links=')):raise RuntimeError('duplicate/wrong native playback start')
            self.node=reference(fields[4].removeprefix('node='))
            self.links={reference(x) for x in fields[5].removeprefix('links=').split(',')}
            if len(self.links)!=2 or native is None:raise RuntimeError('native playback link receipts refused')
            self.native=native;self.active=True;self.started=True;self.sample(force=True)
            return True
        if fields==['NATIVE','PLAYBACK','COMPLETE',str(self.pair)] and self.active:
            _,objects=self.dump()
            if any(identity(o) in self.links or identity(o) in self.ports or (o.get('type')==bridge.NODE and identity(o)==self.node)
                   for o in objects if o.get('type') in (bridge.NODE,bridge.LINK,bridge.PORT)):
                raise RuntimeError('native playback creations survived complete marker')
            if not bridge.pcms_closed(self.before['card']):raise RuntimeError('native playback PCM remained open')
            if len(self.observations)<2 or self.observations[-1]['pointer']-self.observations[0]['pointer']<bridge.RATE*3:
                raise RuntimeError('native playback clock progress insufficient')
            self.active=False;self.complete=True
            self.journal.update(physical_progress_verified=True,queue_progress=self.observations[-1]['pointer']-self.observations[0]['pointer'],clock_checks=len(self.observations),owned_adapter_released=True)
            bridge.owner.save(self.root/'journal.json',self.journal)
            return False
        raise RuntimeError('native playback event sequence refused')

    def sample(self,force=False):
        if not self.active or (not force and time.monotonic()-self.last_sample<.2):return
        self.last_sample=time.monotonic()
        if self.native.poll() is not None or self.server.poll() is not None:
            raise RuntimeError('native playback retained owner exited')
        status=bridge.pcm_path(self.before['card']).read_text()
        # Resource closure can precede reading its COMPLETE marker. Never treat
        # a resumed PCM as the same run or manufacture progress during closure.
        if status.strip()=='closed':self.closed=True;return
        if self.closed:raise RuntimeError('native playback PCM reopened without a new owner')
        _,objects=self.dump();self.graph(objects)
        device=Path('/dev/snd/pcmC%dD0p'%self.before['card']).stat()
        if [device.st_dev,device.st_ino,device.st_rdev]!=self.before['pcm_identity']:
            raise RuntimeError('native playback PCM identity changed')
        if bridge.pcm_path(self.before['card'],'c').read_text().strip()!='closed':
            raise RuntimeError('native playback capture unexpectedly active')
        params=bridge.pcm_path(self.before['card']).with_name('hw_params').read_text()
        bridge.alsa.parameters(params,bridge.RATE)
        pointer=bridge.alsa.clock(status,self.server.pid)
        if self.observations and pointer<self.observations[-1]['pointer']:raise RuntimeError('native PCM clock regressed')
        if not self.observations or pointer>self.observations[-1]['pointer']:self.last_advance=time.monotonic()
        elif self.last_advance is not None and time.monotonic()-self.last_advance>2:raise RuntimeError('native PCM clock stalled')
        if bridge.owner.snapshot()!=self.before['snapshot']:raise RuntimeError('desktop changed during native playback')
        observation={'pointer':pointer,'status':status,'hw_params':params}
        self.observations.append(observation)
        bridge.owner.save(self.root/('clock-%03d.json'%len(self.observations)),observation)
    def fail(self,error):
        self.journal.update(error=str(error),clock_checks=len(self.observations),completed=self.complete)
        bridge.owner.save(self.root/'journal.json',self.journal)


class RecoveryObserver:
    """Two bounded native owners, with an independent released-state barrier.

    Never resets a failed observer or carries PCM progress into the next owner.
    The second session requires the explicit GUI resume marker after our pause
    proof; the GUI waits for that proof before clicking its Connect widget.
    """
    def __init__(self,*args):
        self.args=list(args);self.root=args[2];self.root.mkdir(mode=0o700)
        self.round=1;self.paused=False;self.resumed=False
        self.current=self.make(1)
    def make(self,number):
        args=self.args.copy();args[2]=self.root/('session-%d'%number)
        return Observer(*args)
    @property
    def native(self):return self.current.native
    @native.setter
    def native(self,value):self.current.native=value
    @property
    def complete(self):return self.round==2 and self.resumed and self.current.complete
    def sample(self):return self.current.sample()
    def filtered(self,objects,native):return self.current.filtered(objects,native)
    def event(self,raw,native):return self.current.event(raw,native)
    def pause(self):
        current=self.current
        if self.round!=1 or self.paused or not current.complete or current.active:
            raise RuntimeError('recovery pause requires completed first native owner')
        if current.native is None or current.native.poll() is not None:
            raise RuntimeError('recovery GUI owner disappeared during pause')
        _,objects=current.dump()
        if any(o.get('type')==bridge.LINK for o in objects):
            raise RuntimeError('private routes survived GUI pause')
        if (not bridge.pcms_closed(current.before['card'])
                or bridge.owner.snapshot()!=current.before['snapshot']
                or bridge.owner.connected_binding(current.selected)!=current.before['binding']):
            raise RuntimeError('released baseline differs at GUI pause')
        self.paused=True
        bridge.owner.save(self.root/'pause-proof.json',{
            'pcm_closed':True,'private_links_absent':True,'desktop_state_exact_equal':True,
            'connected_binding_equal':True,'gui_owner_alive':True})
    def resume(self):
        if self.round!=1 or not self.paused or self.resumed or not self.current.complete:
            raise RuntimeError('recovery resume before verified pause or duplicate resume')
        native=self.current.native
        self.current=self.make(2);self.current.native=native
        self.round=2;self.resumed=True
    def fail(self,error):return self.current.fail(error)
