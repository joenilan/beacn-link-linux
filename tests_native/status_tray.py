"""Actual SNI properties/menu on a disposable D-Bus session, never the desktop bus.

Run only through dbus-run-session. This script creates its own fake tray host.
"""
import hashlib
import json
import os
from pathlib import Path
import select
import subprocess
import sys
import threading
import time
import dbus
import dbus.service
from dbus.mainloop.glib import DBusGMainLoop
from gi.repository import GLib


def main():
    binary, evidence = sys.argv[1:]
    root = Path(evidence)
    root.mkdir(mode=0o700)
    # The owner passes an explicit marker only after creating a private bus.
    if os.environ.get('BEACN_PRIVATE_TRAY_BUS') != '1':
        raise RuntimeError('requires disposable bus owner')
    DBusGMainLoop(set_as_default=True)
    bus = dbus.SessionBus(private=True)
    names = []
    service_name = dbus.service.BusName('org.kde.StatusNotifierWatcher', bus)

    class Watcher(dbus.service.Object):
        @dbus.service.method('org.kde.StatusNotifierWatcher', in_signature='s', out_signature='')
        def RegisterStatusNotifierItem(self, name):
            names.append(str(name))

        @dbus.service.method('org.freedesktop.DBus.Properties', in_signature='s', out_signature='a{sv}')
        def GetAll(self, interface):
            return {'IsStatusNotifierHostRegistered': True, 'ProtocolVersion': dbus.Int32(0),
                    'RegisteredStatusNotifierItems': dbus.Array(names, signature='s')}

        @dbus.service.method('org.freedesktop.DBus.Properties', in_signature='ss', out_signature='v')
        def Get(self, interface, prop):
            return self.GetAll(interface)[prop]

    watcher = Watcher(bus, '/StatusNotifierWatcher')
    loop = GLib.MainLoop()
    thread = threading.Thread(target=loop.run, daemon=True)
    thread.start()
    child = subprocess.Popen([binary], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                             stderr=subprocess.PIPE, text=True, bufsize=1)
    proof = {'binary_sha256': hashlib.sha256(Path(binary).read_bytes()).hexdigest(), 'states': []}

    pending = bytearray()
    def line():
        end = time.monotonic()+5
        while b'\n' not in pending:
            if time.monotonic() > end: raise RuntimeError('tray probe response timed out')
            if select.select([child.stdout], [], [], .05)[0]:
                data = os.read(child.stdout.fileno(), 4096)
                if not data: raise RuntimeError('tray probe exited early')
                pending.extend(data)
        result, _, tail = pending.partition(b'\n');pending[:] = tail
        return result.decode().strip()

    def command(text):
        child.stdin.write(text+'\n'); child.stdin.flush()

    try:
        assert line() == 'READY Some(true)'
        assert len(names) == 1
        client = dbus.SessionBus(private=True)
        item = client.get_object(names[0], '/StatusNotifierItem')
        props = dbus.Interface(item, 'org.freedesktop.DBus.Properties')
        labels = [('gray', 'Link disconnected', (149, 163, 183)),
                  ('amber', 'Link connecting', (246, 190, 76)),
                  ('teal', 'Link connected', (81, 224, 194)),
                  ('red', 'Link needs attention', (248, 104, 113)),
                  ('blue', 'Private test session', (112, 172, 255))]
        for color, label, rgb in labels:
            command(color); assert line() == 'OK'
            end = time.monotonic()+3
            while True:
                values = props.GetAll('org.kde.StatusNotifierItem')
                pixels_ready = all(all(tuple(bytes(icon[2])[n+1:n+4]) == rgb for n in range(0, len(icon[2]), 4)) for icon in values['IconPixmap'])
                if str(values['ToolTip'][3]) == label and pixels_ready: break
                if time.monotonic() > end: raise RuntimeError('tray status did not update')
                time.sleep(.03)
            assert str(values['Status']) == ('NeedsAttention' if color == 'red' else 'Active')
            icons = values['IconPixmap']
            assert [int(i[0]) for i in icons] == [16, 22, 32, 64]
            for width, height, pixels in icons:
                data = bytes(pixels)
                assert len(data) == int(width)*int(height)*4
                assert any(data[n] == 255 for n in range(0, len(data), 4))
                assert all(tuple(data[n+1:n+4]) == rgb for n in range(0, len(data), 4)), (color, rgb, list(sorted(set(tuple(data[n:n+4]) for n in range(0, len(data), 4))))[:18])
            proof['states'].append(color)
        dbus.Interface(item, 'org.kde.StatusNotifierItem').Activate(0, 0)
        command('actions'); assert line() == 'ACTIONS 1'; assert line() == 'OK'
        menu = dbus.Interface(client.get_object(names[0], values['Menu']), 'com.canonical.dbusmenu')
        _, layout = menu.GetLayout(0, -1, [])
        entries = {str(node[1].get('label', '')): int(node[0]) for node in layout[2]}
        menu.Event(entries['Stop link'], 'clicked', dbus.Int32(0), dbus.UInt32(0))
        command('actions'); assert line() == 'ACTIONS 2'; assert line() == 'OK'
        menu.Event(entries['Quit'], 'clicked', dbus.Int32(0), dbus.UInt32(0))
        command('actions'); assert line() == 'ACTIONS 4'; assert line() == 'OK'
        command('image '+str(root/'icons.ppm')); assert line() == 'OK'
        command('quit'); assert line() == 'STOPPED'
        child.wait(timeout=4)
        assert child.returncode == 0 and not child.stderr.read()
        assert not client.name_has_owner(names[0])
        proof.update(actions_verified=True, bus_name_released=True, exit=0)
    finally:
        if child.poll() is None: child.kill(); child.wait(timeout=3)
        loop.quit(); thread.join(timeout=2)
        (root/'journal.json').write_text(json.dumps(proof, indent=2))
    print(json.dumps(proof))


if __name__ == '__main__':
    main()
