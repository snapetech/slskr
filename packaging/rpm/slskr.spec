Name:           slskr
Version:        0.2.40
Release:        1%{?dist}
Summary:        Rust Soulseek daemon with bundled Web UI
License:        AGPL-3.0-only
URL:            https://github.com/snapetech/slskr
Source0:        slskr-v0.2.40-x86_64-unknown-linux-gnu.tar.gz
Source1:        slskr-v0.2.40-aarch64-unknown-linux-gnu.tar.gz
Source2:        slskr.service
Source3:        slskr.sysusers
Source4:        slskr.tmpfiles

%global debug_package %{nil}

ExclusiveArch:  x86_64 aarch64
BuildRequires:  systemd-rpm-macros
Requires(pre):  shadow-utils
%{?systemd_requires}
%{!?_unitdir:%global _unitdir %{_prefix}/lib/systemd/system}
%{!?_tmpfilesdir:%global _tmpfilesdir %{_prefix}/lib/tmpfiles.d}
%{!?_sysusersdir:%global _sysusersdir %{_prefix}/lib/sysusers.d}

%ifarch x86_64
%global slskr_target x86_64
%global slskr_source %{SOURCE0}
%elifarch aarch64
%global slskr_target aarch64
%global slskr_source %{SOURCE1}
%else
%{error:unsupported architecture}
%endif

%description
slskr is a Rust Soulseek daemon with an HTTP API and bundled Web UI.

%prep
mkdir -p slskr-v%{version}-%{slskr_target}-unknown-linux-gnu
tar -xzf %{slskr_source} \
    --strip-components=1 \
    -C slskr-v%{version}-%{slskr_target}-unknown-linux-gnu
cd slskr-v%{version}-%{slskr_target}-unknown-linux-gnu

%build

%install
cd slskr-v%{version}-%{slskr_target}-unknown-linux-gnu
install -Dm755 slskr %{buildroot}%{_bindir}/slskr
install -Dm644 docs/slskr.config.example.toml %{buildroot}%{_sysconfdir}/slskr/config.toml
install -Dm644 %{SOURCE2} %{buildroot}%{_unitdir}/slskr.service
install -Dm644 %{SOURCE3} %{buildroot}%{_sysusersdir}/slskr.conf
install -Dm644 %{SOURCE4} %{buildroot}%{_tmpfilesdir}/slskr.conf
install -Dm644 README.md %{buildroot}%{_docdir}/slskr/README.md
install -Dm644 LICENSE %{buildroot}%{_licensedir}/slskr/LICENSE
mkdir -p %{buildroot}%{_datadir}/slskr/web
cp -R web/build %{buildroot}%{_datadir}/slskr/web/build

%pre
%sysusers_create_compat %{SOURCE3}

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
