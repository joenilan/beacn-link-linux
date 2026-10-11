# Verified static findings

Target: BEACNLink.exe 1.0.4; size 5255096 bytes.
SHA256: 74760CE85BEE8620C5BF81F5951FECFE20A04EDF163129361C6890D5481DB64C.
Authenticode: valid, Beacon Hill Innovations Ltd.

AMD64 PE32+, no CLR header. Embedded JUCE 7.0.6; MSVC-style RTTI;
PE linker 14.40. No COFF symbols, exports, RSDS record or PDB path found.
Debug entry type 13 is consistent with POGO metadata.

Imports include SetupDi interface enumeration/detail APIs, CreateFileW,
DeviceIoControl, WinUsb Initialize/Free/ReadPipe/WritePipe/SetPipePolicy/
SetCurrentAlternateSetting/ResetPipe/AbortPipe, process identification,
CoCreateInstance and CoInitializeEx.

Preferred VA 0x14013296D begins stack construction of the verified Link GUID.
0x1401329C9/0x1401329CF load PID 4003 and VID 33AE; call at 0x1401329E1
targets 0x1400075F0. This is a tracing anchor, not a decoded protocol.

RTTI file offsets: WindowsAudioSessionDiscovery 0x4ADB00;
BeacnStudioCompanionUsbDevice 0x4ADDD0; WinUsbDevice 0x4ADE08;
UsbDeviceConnectionManager 0x4ADEA0; UsbDeviceScanner 0x4ADF08.
No BEACN/TUSBAudio imports or delay imports; matching API-name strings absent.
Dynamic usage remains possible. Tools: GNU objdump/strings, pefile and capstone.

Protocol undecoded; routing API calls unresolved; native Linux runtime untested.
