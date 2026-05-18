Name:           slskr
Version:        0.2.15
Release:        1%{?dist}
Summary:        Rust Soulseek daemon with bundled Web UI
License:        AGPL-3.0-only
URL:            https://github.com/snapetech/slskr
Source0:        slskr-v0.2.15-x86_64-unknown-linux-gnu.tar.gz
Source1:        slskr.service
Source2:        slskr.sysusers
Source3:        slskr.tmpfiles

BuildArch:      x86_64
BuildRequires:  systemd-rpm-macros
Requires(pre):  shadow-utils
%{?systemd_requires}

%description
slskr is a Rust Soulseek daemon with an HTTP API and bundled Web UI.

%prep
%setup -q -n slskr-v%{version}-x86_64-unknown-linux-gnu

%build

%install
install -Dm755 slskr %{buildroot}%{_bindir}/slskr
install -Dm644 docs/slskr.config.example.toml %{buildroot}%{_sysconfdir}/slskr/config.toml
install -Dm644 %{SOURCE1} %{buildroot}%{_unitdir}/slskr.service
install -Dm644 %{SOURCE2} %{buildroot}%{_sysusersdir}/slskr.conf
install -Dm644 %{SOURCE3} %{buildroot}%{_tmpfilesdir}/slskr.conf
install -Dm644 README.md %{buildroot}%{_docdir}/slskr/README.md
install -Dm644 LICENSE %{buildroot}%{_licensedir}/slskr/LICENSE
mkdir -p %{buildroot}%{_datadir}/slskr/web
cp -R web/build %{buildroot}%{_datadir}/slskr/web/build

%pre
%sysusers_create_compat %{SOURCE2}

%post
%systemd_post slskr.service
%tmpfiles_create %{_tmpfilesdir}/slskr.conf

%preun
%systemd_preun slskr.service

%postun
%systemd_postun_with_restart slskr.service

%files
%license %{_licensedir}/slskr/LICENSE
%doc %{_docdir}/slskr/README.md
%{_bindir}/slskr
%config(noreplace) %{_sysconfdir}/slskr/config.toml
%{_unitdir}/slskr.service
%{_sysusersdir}/slskr.conf
%{_tmpfilesdir}/slskr.conf
%{_datadir}/slskr/web/build
