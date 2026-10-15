import copy
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'tests_native'))
import native_playback_observer as witness
from test_studio_pipewire_bridge import graph


def fixture():
    objects,target=graph()
    def obj(id,kind,p):
        return {'id':id,'type':'PipeWire:Interface:'+kind,'info':{'state':'active','props':{'object.serial':id+100,**p}}}
    objects.append(obj(7,'Node',{'client.id':4,'beacn.link.owner':'owned','api.alsa.path':'hw:2,0'}))
    for id,channel in [(14,'FL'),(15,'FR')]:
        objects.append(obj(id,'Port',{'node.id':6,'port.direction':'out','audio.channel':channel,'port.monitor':True}))
    for i in range(8):
        objects.append(obj(40+i,'Port',{'node.id':7,'port.direction':'in','audio.channel':'AUX%d'%i}))
    for id,out,input_ in [(30,14,40),(31,15,41)]:
        objects.append(obj(id,'Link',{'client.id':4,'link.output.node':6,'link.input.node':7,'link.output.port':out,'link.input.port':input_,'beacn.link.owner':'owned'}))
    return objects,target


class NativePlaybackWitnessTests(unittest.TestCase):
    def observer(self,root):
        objects,target=fixture()
        observer=witness.Observer(None,{'card':2},root,lambda:(None,objects),SimpleNamespace(),SimpleNamespace(pid=33),SimpleNamespace(pid=22),target,'owned',1)
        observer.native=SimpleNamespace(pid=44,poll=lambda:None)
        observer.node=(7,107);observer.links={(30,130),(31,131)}
        return observer,objects

    def test_reference_bounds_and_ascii_are_strict(self):
        self.assertEqual(witness.reference('7:107'),(7,107))
        for invalid in ['0:1','1:0','-1:2','1:2:3','x:4','１:2',str(2**32)+':4','1:'+str(2**64)]:
            with self.subTest(invalid=invalid),self.assertRaises(RuntimeError):witness.reference(invalid)

    def test_exact_mapping_generation_client_direction_and_no_foreign_auxiliary_link(self):
        with tempfile.TemporaryDirectory() as folder:
            observer,objects=self.observer(Path(folder)/'evidence')
            observer.graph(objects)
            self.assertEqual([o['id'] for o in observer.filtered(objects,observer.native) if o['type']==witness.bridge.LINK],[20,21])
            for id,key,value in [(7,'api.alsa.path','hw:3,0'),(7,'client.id',2),(30,'client.id',2),(30,'object.serial',999),
                                 (14,'port.monitor',False),(14,'port.direction','in'),(40,'audio.channel','AUX7'),(40,'object.serial',999)]:
                changed=copy.deepcopy(objects);next(o for o in changed if o['id']==id)['info']['props'][key]=value
                with self.subTest(id=id,key=key),self.assertRaises(RuntimeError):observer.graph(changed)

    def test_marker_order_prefixes_and_duplicate_start_refuse_before_pcm_sampling(self):
        with tempfile.TemporaryDirectory() as folder:
            observer,_=self.observer(Path(folder)/'evidence')
            with patch.object(observer,'sample') as sample:
                for bad in [b'NATIVE PLAYBACK COMPLETE 1',b'NATIVE PLAYBACK ACTIVE 2 node=7:107 links=30:130,31:131',
                            b'NATIVE PLAYBACK ACTIVE 1 7:107 links=30:130,31:131',b'NATIVE PLAYBACK ACTIVE 1 node=7:107 links=30:130,30:130']:
                    with self.subTest(bad=bad),self.assertRaises(RuntimeError):observer.event(bad,observer.native)
                # A refused malformed marker does not authorize a new session.
                self.assertFalse(sample.called)

if __name__=='__main__':unittest.main()
