# Verified inventory

Read-only Windows observations, 2026-10-11. Machine identifiers are omitted.

- Windows 11 Pro x64, build 26200.
- BEACN Link App uninstall version 1.0.4.0; BEACNLink.exe file/product 1.0.4.
- Install root: C:\Program Files\BEACN\BEACN Link App.
- Studio driver package 5.68.0; device driver version 5.68.0.11769.
- Present Studio USB VID/PID 33AE:4003, revision 0100.
- MI_00 uses BEACNStudioUsbAudio (oem5.inf); its KS child uses
  BEACNStudioUsbAudioks (oem4.inf). Deployed SYS file versions: 5.68.0.0.
- MI_03 uses Microsoft WINUSB, winusb.inf, version 10.0.26100.9444.
- Link interface GUID: {BB9A2EE6-D5EF-4AB7-AA23-85B2F6DDF006}.
- Four active capture and four render Link endpoints; stored format is
  stereo 96 kHz, 32-bit integer PCM, channel mask 3.
- No ordinary Windows service matched BEACN by name or executable path.

Stored formats do not establish active streams or every supported format.
Absence of a named service does not locate the discovery/routing logic.
