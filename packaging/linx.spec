# Binary RPM from a staged tree; built by packaging/release.sh.
%global debug_package %{nil}
%global __strip /bin/true

Name:           linx
Version:        %{linx_version}
Release:        1%{?dist}
Summary:        Unofficial BEACN Link app for Linux
License:        GPL-3.0-only
URL:            https://github.com/joenilan/beacn-link-linux
Requires:       pipewire >= 1.0
Requires:       wireplumber
Requires:       libusb1 >= 1.0.27
# Loaded at runtime by the GPUI window system (not visible to the ELF scan).
Requires:       vulkan-loader
Requires:       libwayland-client
Requires:       libwayland-egl
Recommends:     mesa-vulkan-drivers

%description
LinX gives a Linux gaming PC BEACN Studio's four Link channels: applications
play to "LinX Link Out" .. "LinX Link 4 Out", the Studio's return pairs arrive
as "LinX Link In" .. "LinX Link 4 In", and BEACN App on the Windows streaming
PC sees each application and can move it. Unofficial; not affiliated with
BEACN.

%install
cp -a %{linx_stage}/. %{buildroot}/

%post
udevadm control --reload-rules >/dev/null 2>&1 || :
udevadm trigger --subsystem-match=usb --attr-match=idVendor=33ae >/dev/null 2>&1 || :

%postun
udevadm control --reload-rules >/dev/null 2>&1 || :

%files
/usr/bin/linx
/usr/share/applications/io.github.linx.desktop
/usr/share/icons/hicolor/scalable/apps/io.github.linx.svg
/usr/lib/udev/rules.d/70-linx-beacn-studio.rules
%license /usr/share/licenses/linx/LICENSE
%license /usr/share/licenses/linx/THIRD_PARTY_NOTICES.txt
%doc /usr/share/doc/linx/README.md
