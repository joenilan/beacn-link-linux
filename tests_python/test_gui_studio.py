import importlib.util
from pathlib import Path
import unittest
import os
import subprocess
import sys
from types import SimpleNamespace

spec=importlib.util.spec_from_file_location('gui_studio',Path(__file__).resolve().parents[1]/'tests_native/gui_studio.py')
gui=importlib.util.module_from_spec(spec);spec.loader.exec_module(gui)

class StudioGuiTests(unittest.TestCase):
    def test_requires_full_explicit_hardware_scope_before_any_child(self):
        gui.validate('run',True,True,True,40,None,False)
        for args in [('routing-probe',True,True,True,40,None,False),('run',False,True,True,40,None,False),
                     ('run',True,False,True,40,None,False),('run',True,True,False,40,None,False),
                     ('run',True,True,True,39,None,False),('run',True,True,True,40,'service',False),
                     ('run',True,True,True,40,None,True)]:
            with self.subTest(args=args),self.assertRaises(RuntimeError):gui.validate(*args)

    def test_wrong_pair_early_late_duplicate_and_stop_before_start_refuse(self):
        for line,pair,count,started,active in [(b'PHYSICAL START 2',1,5,False,False),
                (b'PHYSICAL START 1',1,0,False,False),(b'PHYSICAL START 1',1,32,False,False),
                (b'PHYSICAL START 1',1,5,True,True),(b'PHYSICAL STOP 1',1,5,False,False),
                (b'PHYSICAL STOP 2',1,5,True,True),(b'PHYSICAL START 1 extra',1,5,False,False)]:
            with self.subTest(line=line,count=count),self.assertRaises(RuntimeError):
                gui.physical_request(line,pair,count,40,started,active)

    def test_valid_pairs_have_one_start_and_owned_stop(self):
        for pair in range(1,5):
            self.assertTrue(gui.physical_request(f'PHYSICAL START {pair}'.encode(),pair,4,40,False,False))
            self.assertFalse(gui.physical_request(f'PHYSICAL STOP {pair}'.encode(),pair,39,40,True,True))

    def test_recovery_cannot_launch_supervisor_other_pair_or_unbounded_scope(self):
        for observer,pair,cycles in [(None,1,40),(object(),2,40),(object(),1,80)]:
            with self.subTest(pair=pair,cycles=cycles),self.assertRaises(RuntimeError):
                gui.execute(None,None,None,None,None,pair,cycles,None,None,None,None,
                    observer=observer,recovery=True)

    @unittest.skipUnless(sys.platform=='linux','Linux pidfd process ownership')
    def test_actual_owned_process_is_pinned_and_stopped_via_pidfd(self):
        child=subprocess.Popen(['/usr/bin/sleep','30'])
        try:
            owned=gui.NativeChild(child.pid,SimpleNamespace(pid=os.getpid()),Path('/usr/bin/sleep'))
            self.assertIsNone(owned.poll())
            self.assertTrue(owned.close())
            child.wait(timeout=3)
        finally:
            if child.poll() is None:child.kill();child.wait(timeout=3)

    @unittest.skipUnless(sys.platform=='linux','Linux process ancestry')
    def test_an_unrelated_wrapper_cannot_claim_a_process(self):
        child=subprocess.Popen(['/usr/bin/sleep','30'])
        try:
            with self.assertRaises(RuntimeError):gui.NativeChild(child.pid,SimpleNamespace(pid=99999999),Path('/usr/bin/sleep'))
        finally:child.terminate();child.wait(timeout=3)

if __name__=='__main__':unittest.main()
