Name:           mailer
Version:        0.2.0
Release:        1%{?dist}
Summary:        GTK4/Libadwaita email client
License:        MIT
URL:            https://github.com/sayler/mailer

AutoReqProv:    no

Requires:       gtk4
Requires:       libadwaita
Requires:       webkit6
Requires:       gnome-keyring

%description
A modern email client built with GTK4 and Libadwaita.
Supports IMAP/SMTP with TLS, multiple accounts, HTML rendering,
message threading, drafts, signatures, and desktop notifications.

%install
install -Dm755 %{_project_dir}/target/release/mailer \
    %{buildroot}%{_bindir}/mailer
install -Dm644 %{_project_dir}/data/icons/com.sayler.mailer.png \
    %{buildroot}%{_datadir}/icons/hicolor/256x256/apps/com.sayler.mailer.png
install -Dm644 %{_project_dir}/data/com.sayler.mailer.desktop \
    %{buildroot}%{_datadir}/applications/com.sayler.mailer.desktop

%files
%{_bindir}/mailer
%{_datadir}/icons/hicolor/256x256/apps/com.sayler.mailer.png
%{_datadir}/applications/com.sayler.mailer.desktop

%post
gtk-update-icon-cache -f -t %{_datadir}/icons/hicolor/ 2>/dev/null || :

%postun
gtk-update-icon-cache -f -t %{_datadir}/icons/hicolor/ 2>/dev/null || :
