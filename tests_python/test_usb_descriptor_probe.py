import importlib.util
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("usb_descriptor_probe", ROOT / "tools/usb_descriptor_probe.py")
probe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(probe)


@unittest.skipUnless(os.name == "posix", "sysfs directory fixtures require POSIX names")
class ProbeTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / "synthetic-usb"
        self.root.mkdir()
        self.raw = bytes.fromhex((ROOT / "fixtures/usb-composite.synthetic.hex").read_text())
        (self.root / "descriptors").write_bytes(self.raw)
        for name, value in {"idVendor": "33ae", "idProduct": "4003", "bcdDevice": "0100",
                            "bConfigurationValue": "7", "serial": "MUST-NOT-READ"}.items():
            (self.root / name).write_text(value)
        for number, cls in ((0, "01"), (3, "ff")):
            directory = self.root / (self.root.name + ":7." + str(number))
            directory.mkdir()
            for name, value in {"bInterfaceNumber": f"{number:02x}", "bAlternateSetting": " 0",
                                "bInterfaceClass": cls}.items():
                (directory / name).write_text(value)

    def invoke(self):
        codec = os.environ.get("BEACN_TEST_PLANNER_BIN")
        if not codec:
            self.fail("BEACN_TEST_PLANNER_BIN must name the actual built CLI")
        return probe.probe(self.root, codec, fixture=True)

    def test_actual_cli_fixture_reports_active_state_without_touching_serial_or_files(self):
        before = {str(p.relative_to(self.root)): p.read_bytes() for p in self.root.rglob("*") if p.is_file()}
        original = probe.cached
        def checked(path, limit):
            self.assertNotIn(path.name, ("serial", "product", "manufacturer"))
            return original(path, limit)
        with patch.object(probe, "cached", side_effect=checked):
            report = self.invoke()
        self.assertEqual(report["mode"], "synthetic_fixture")
        self.assertEqual(report["transport_gate"], "requires_authorized_alternate_change")
        self.assertNotIn("MUST-NOT-READ", str(report))
        self.assertEqual(before, {str(p.relative_to(self.root)): p.read_bytes() for p in self.root.rglob("*") if p.is_file()})

    def test_state_change_during_validator_aborts_observation(self):
        original = subprocess.run
        def changing(*args, **kwargs):
            result = original(*args, **kwargs)
            (self.root / "bConfigurationValue").write_text("8")
            return result
        with patch.object(probe.subprocess, "run", side_effect=changing):
            with self.assertRaises(probe.ProbeError): self.invoke()

    def test_wrong_identity_missing_interface_oversize_and_live_directory_refused(self):
        with self.assertRaises(probe.ProbeError): probe.probe(self.root, "unused", fixture=False)
        (self.root / "idProduct").write_text("4004")
        with self.assertRaises(probe.ProbeError): self.invoke()
        (self.root / "idProduct").write_text("4003")
        (self.root / "descriptors").write_bytes(self.raw + bytes(probe.MAX_BYTES))
        with self.assertRaises(probe.ProbeError): self.invoke()
        (self.root / "descriptors").write_bytes(self.raw)
        (self.root / (self.root.name + ":7.3") / "bInterfaceNumber").write_text("04")
        with self.assertRaises(probe.ProbeError): self.invoke()

    def test_malformed_descriptor_is_refused_by_actual_rust_validator(self):
        (self.root / "descriptors").write_bytes(self.raw[:-1])
        with self.assertRaises(probe.ProbeError): self.invoke()

    @unittest.skipUnless(os.name == "posix", "driver-link fixture requires POSIX symlinks")
    def test_bound_kernel_driver_is_reported_as_blocked_without_detaching(self):
        driver = self.root / (self.root.name + ":7.3") / "driver"
        driver.symlink_to("/sys/bus/usb/drivers/usb_test_driver")
        self.assertEqual(self.invoke()["transport_gate"], "blocked_kernel_driver")
        self.assertTrue(driver.is_symlink())


if __name__ == "__main__":
    unittest.main()
