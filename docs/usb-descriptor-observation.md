# First physical Studio descriptor observation

Observed 2026-10-12 on the existing Ubuntu 26.04.1 VM. The owner explicitly
authorized descriptor-only testing, managed Studio's VMware connection and return,
and confirmed disconnection afterward. No protocol writes, alternate-setting
changes, driver detach, audio capture or routing commands were issued by the agent.
Normal kernel enumeration/audio-driver binding occurred during attachment.

## Direct observations

Exactly one guest target matched VID/PID 33AE:4003. The cached device descriptor
reports bcdUSB 0200 and bcdDevice 0100; this revision field is not a decoded firmware
version. The active configuration value was 1 and sysfs reported speed 480 Mb/s.
The VM UI/controller had previously been reported as USB 1.1; the cached speed
observation does not establish measured throughput or a production controller setup.
The earlier ideal-controller warning and timed-out query remain separate evidence.

The unchanged built CLI and sysfs cache probe accepted the physical descriptor
stream. Both validator calls exited zero. The bounded observation took 0.122 seconds.
The probe's before/after cached metadata comparison matched. It read no serial,
manufacturer or product string attributes; raw cache and reports remain private.

| Interface number | Current alternate | Class/subclass/protocol | Bound driver | Advertised alternate 1 |
| --- | --- | --- | --- | --- |
| 0 | 0 | 01/01/20 | snd-usb-audio | No alternate 1 observed |
| 1 | 0 | 01/02/20 | snd-usb-audio | OUT 02, attributes 05, max packet 416, interval 1 |
| 2 | 0 | 01/02/20 | snd-usb-audio | IN 82, attributes 25, max packet 416, interval 1 |
| 3 | 0 | FF/00/00 | None | OUT 03 and IN 83, attributes 02, max packet 512 each, interval 0 |

Interface 0 alternate 0 advertises endpoint 81, attributes 03, max packet 6,
interval 8. Interfaces 1, 2 and 3 alternate 0 advertise no endpoints.
The low two endpoint-attribute bits identify interrupt, isochronous and bulk
types using the [USB Chapter 9 definitions](https://github.com/torvalds/linux/blob/master/include/uapi/linux/usb/ch9.h).
Class-specific audio descriptor bodies were skipped by the validator; no PCM
format/channel layout, feedback behavior or audio interoperability is inferred.

This establishes configuration 1 / vendor interface 3 / alternate 1 / bulk 03+83
as an actual advertised layout, corroborating the static transport candidates.
It does not establish endpoint reachability while alternate 0 is active.
The probe correctly reported `requires_authorized_alternate_change`.
The same interface number, class and endpoints are no longer merely assumptions
for this inspected device/cache; broader device revisions remain untested.

## Return and settings comparison

After the owner's disconnection, the guest had zero matching targets. Windows
reported the same physical target with all three USB functions status OK.
The private host comparison found identical endpoint IDs/states, six defaults,
mix formats/channel masks and master/channel volumes/mutes. The endpoint-reference
string changed in the earlier warning snapshot had returned to its baseline value.

The complete Windows baseline did not match: session count increased from 26 to
30, the MMDevices tree retained 16 changed binary values in property family
194EF948-7CDB-403E-9F47-19418F7B24FD, and persisted user policy gained one key/value.
User audio settings were identical. These differences were preserved privately;
their meanings/causes are not all established and no registry values were rewritten.
Concurrent session changes must not be treated automatically as owned changes.

The guest returned to 5 nodes, 8 ports, zero links/devices and 5 metadata objects.
Three nodes and seven ports were recreated; another port's parent identity changed.
The recreated logical nodes were auto_null, Midi-Bridge and bluez_midi.server.
Node properties/parameters match after excluding object/client identities; port
settings match after mapping their logical parents. Metadata/defaults, links and
devices match exactly. Numeric identities are still new: this comparison does
not authorize reuse of old execution plans/receipts.

WirePlumber's `stream-properties` hash changed; the other three tracked state hashes
matched. The after-file contains auto_null, Firefox and notification entries.
The baseline contains hashes rather than a full copy of that file, so its previous
contents were not reconstructed or blindly replaced. The agent issued no guest
routing, metadata, volume or persistence setters. Future physical-test baselines
must preserve relevant state-file contents as well as hashes before mutation.

**Outcome:** the physical cached-descriptor milestone passed and Studio was
returned to Windows. Observed logical audio controls/defaults match, but this was
not an exact persistent-state or object-identity restoration. Do not describe it
as a completely unchanged desktop/guest or a working USB transport.

## Evidence provenance

Cached descriptor SHA256:
`75b6aa814baa54f2b19c292289a107411797ec3dec6a344b38b756471364dae7`.
Private descriptor observation journal SHA256:
`c490cecfc2da667211d425eb3d2cb68b4c6541f94a7d0b072576e89483a8ad0d`.
Executed Linux CLI SHA256:
`5ae88c450600d47ccd8ef5687a779037250d4150ed8ac9f7057223f51ebec73a`.
Probe SHA256:
`646b804ed9d71c9768970f75be6e19b8e94dc5b791e4cb4011d520342b5764bd`.
Both were pinned against the saved guest preflight before execution, using tested
code tree `4ef14186053efc7f79599aecac2cc314be52de6bb8045d8113aa7907d44bb64b`.
No source changes or new builds were needed for this authorized observation.

Returned Windows snapshot SHA256:
`28d3b99c5b72c9e6c5b432dfd6d3f20d8789855a94d99949e65314d9a480fafd`.
Returned guest snapshot SHA256:
`4530554c00feb104da6ce810b36a9d840d6212d406eb99971407574919f5d4f2`.
After `stream-properties` SHA256:
`ed030cee85f7a88a1e087bdc3833d9514a7b2fb28926c042190ff6d1548d0a36`.
Fourteen private guest evidence files were copied to the existing host evidence
area through the dedicated verified SSH connection; hashes were recorded.
No endpoint GUIDs, serials, host addresses, raw descriptor bytes or captures are
committed. Previous evidence and settings snapshots remain intact.

## Next implementation and authority boundary

Implement an optional selected-device Linux transport for the existing session
controller, using the confirmed advertised profile and preserving audio-interface
ownership. Keep it disabled in the normal CLI, refuse kernel-driver ownership,
auto-detach, resets and fallback, and compile/test the exact software tree offline.
Before another physical test, preserve full state-file backups and resolve the
controller configuration and baseline-comparison limits above.

A bounded claim of interface 3, alternate 1 selection and A0/A1/A2 handshake needs
separate explicit authorization once that implementation and recovery plan are
reviewable. Current permission was descriptor-only and has been completed.
Application publication, assignment ingestion, real routing/audio and end-to-end
Windows visibility remain later milestones, followed by native-Linux acceptance.
