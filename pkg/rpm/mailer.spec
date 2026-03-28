Name:           mailer
Version:        0.1.0
Release:        1%{?dist}
Summary:        Email client
License:        MIT
URL:            https://github.com/sayler/mailer

AutoReqProv:    no

%description
GTK4/Libadwaita email client.

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
