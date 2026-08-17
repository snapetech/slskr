#!/usr/bin/env python3
"""Emit executable evidence for the operator-packaging parity ledger.

This audit deliberately handles only artifact families whose contracts can be
checked from the checked-in files without pretending that a package-manager or
container daemon was installed on the host.  Unsupported families and
unsupported lifecycle cases are emitted as failed rows and remain open in the
parity manifest.
"""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path


CASES = (
    "build-render-and-artifact-contents",
    "fresh-install-and-upgrade",
    "start-stop-signal-and-restart",
    "configuration-user-permissions-and-secrets",
    "network-ports-storage-and-health",
    "failure-rollback-uninstall-and-logs",
)


def operator_families(root: Path) -> dict[str, list[str]]:
    families: dict[str, set[str]] = {}

    dockerfile = root / "Dockerfile"
    if dockerfile.is_file():
        families.setdefault("container-root", set()).add("Dockerfile")

    workflow_root = root / ".github/workflows"
    if workflow_root.is_dir():
        for path in sorted(workflow_root.glob("*.y*ml")):
            families.setdefault(f"github-workflow-{path.stem}", set()).add(
                str(path.relative_to(root))
            )

    packaging_root = root / "packaging"
    if packaging_root.is_dir():
        for path in sorted(packaging_root.rglob("*")):
            if path.is_file():
                relative = path.relative_to(root)
                if len(relative.parts) > 1:
                    families.setdefault(f"packaging-{relative.parts[1]}", set()).add(
                        str(relative)
                    )

    systemd_root = root / "etc/systemd"
    if systemd_root.is_dir():
        for path in sorted(systemd_root.rglob("*")):
            if path.is_file():
                families.setdefault("systemd-hardened", set()).add(
                    str(path.relative_to(root))
                )

    nix_file = root / "flake.nix"
    if nix_file.is_file():
        families.setdefault("nix-root", set()).add("flake.nix")

    vpn_root = root / "src/slskdN.VpnAgent"
    if vpn_root.is_dir():
        for path in sorted(vpn_root.rglob("*")):
            if path.is_file() and (
                path.name == "install.sh" or "systemd" in path.relative_to(vpn_root).parts
            ):
                families.setdefault("vpn-agent", set()).add(str(path.relative_to(root)))

    return {family: sorted(paths) for family, paths in sorted(families.items())}


def text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8-sig")
    except UnicodeDecodeError:
        # Package trees may contain icons, archives, or other binary payloads.
        # They cannot prove a text-token contract, but they must not abort the
        # complete operator inventory before the remaining files are audited.
        return ""


def all_present(source: str, tokens: tuple[str, ...]) -> bool:
    return all(token in source for token in tokens)


def aur_cases(local_root: Path, target_root: Path) -> dict[str, bool]:
    local_pkgbuild = text(local_root / "packaging/aur/PKGBUILD")
    target_pkgbuild = text(target_root / "packaging/aur/PKGBUILD")
    local_install = text(local_root / "packaging/aur/slskr.install")
    target_install = text(target_root / "packaging/aur/slskd.install")
    local_service = text(local_root / "packaging/aur/slskr.service")
    target_service = text(target_root / "packaging/aur/slskd.service")
    local_sysusers = text(local_root / "packaging/aur/slskr.sysusers")
    target_sysusers = text(target_root / "packaging/aur/slskd.sysusers")
    local_tmpfiles = text(local_root / "packaging/aur/slskr.tmpfiles")
    target_tmpfiles = text(target_root / "packaging/aur/slskd.tmpfiles")
    return {
        "build-render-and-artifact-contents": all(
            (
                all_present(
                    source,
                    ("build()", "package()", "install=", "sha256sums="),
                )
                for source in (local_pkgbuild, target_pkgbuild)
            )
        ),
        "fresh-install-and-upgrade": all(
            all_present(source, ("post_install()", "post_upgrade()"))
            for source in (local_install, target_install)
        ),
        "start-stop-signal-and-restart": all(
            all_present(
                source,
                ("ExecStart=", "Restart=on-failure", "WantedBy=multi-user.target"),
            )
            for source in (local_service, target_service)
        ),
        "configuration-user-permissions-and-secrets": all(
            all_present(source, ("/var/lib/", "/etc/", "User=", "Group="))
            for source in (local_service, target_service)
        )
        and all(
            all_present(source, ("/var/lib/",))
            for source in (local_sysusers, target_sysusers, local_tmpfiles, target_tmpfiles)
        ),
        "network-ports-storage-and-health": False,
        "failure-rollback-uninstall-and-logs": all(
            all_present(source, ("post_remove", "systemctl daemon-reload"))
            for source in (local_install, target_install)
        ),
    }


def debian_cases(local_root: Path, target_root: Path) -> dict[str, bool]:
    local_control = text(local_root / "packaging/debian/control")
    target_control = text(target_root / "packaging/debian/control")
    local_rules = text(local_root / "packaging/debian/rules")
    target_rules = text(target_root / "packaging/debian/rules")
    local_postinst = text(local_root / "packaging/debian/postinst")
    target_postinst = text(target_root / "packaging/debian/postinst")
    return {
        "build-render-and-artifact-contents": all(
            all_present(source, ("Source:", "Package:", "Build-Depends:"))
            for source in (local_control, target_control)
        )
        and all(
            all_present(source, ("%:", "override_dh_auto_install:"))
            for source in (local_rules, target_rules)
        ),
        "fresh-install-and-upgrade": all(
            all_present(source, ("set -e", "#DEBHELPER#"))
            for source in (local_postinst, target_postinst)
        ),
        "start-stop-signal-and-restart": all(
            "lib/systemd/system" in source
            for source in (local_rules, target_rules)
        ),
        "configuration-user-permissions-and-secrets": (
            all("/etc/" in source for source in (local_rules, target_rules))
            and all(
                token in "\n".join((local_rules, target_rules, local_postinst, target_postinst))
                for token in ("sysusers", "tmpfiles")
            )
        ),
        "network-ports-storage-and-health": False,
        "failure-rollback-uninstall-and-logs": False,
    }


def homebrew_cases(local_root: Path, target_root: Path) -> dict[str, bool]:
    local = text(local_root / "packaging/homebrew/Formula/slskr.rb")
    target = text(target_root / "packaging/homebrew/Formula/slskdn.rb")
    return {
        "build-render-and-artifact-contents": all_present(
            local, ("class Slskr", "def install", "test do", "sha256")
        )
        and all_present(target, ("class Slskdn", "def install", "test do", "sha256")),
        "fresh-install-and-upgrade": all_present(
            local, ("def install", "test do")
        )
        and all_present(target, ("def install", "test do")),
        "start-stop-signal-and-restart": False,
        "configuration-user-permissions-and-secrets": False,
        "network-ports-storage-and-health": False,
        "failure-rollback-uninstall-and-logs": False,
    }


def container_cases(local_root: Path, target_root: Path) -> dict[str, bool]:
    local = text(local_root / "Dockerfile")
    target = text(target_root / "Dockerfile")
    return {
        "build-render-and-artifact-contents": (
            local.count("\nFROM ") + local.startswith("FROM ") >= 2
            and all_present(
                local,
                (
                    "cargo build --release -p slskr",
                    "COPY --from=web-builder",
                    "COPY --from=builder",
                    "/usr/local/bin/slskr",
                    "/usr/share/slskr/web/build",
                    "/etc/slskr/config.toml.example",
                ),
            )
            and "FROM " in target
            and "COPY --from=" in target
        ),
        "fresh-install-and-upgrade": False,
        "start-stop-signal-and-restart": all_present(local, ("ENTRYPOINT", "CMD"))
        and all_present(target, ("ENTRYPOINT", "CMD")),
        "configuration-user-permissions-and-secrets": (
            all_present(
                local,
                (
                    "USER slskr",
                    "SLSKR_STATE_DIR=/var/lib/slskr",
                    "chown -R slskr:slskr /var/lib/slskr",
                    "config.toml.example",
                ),
            )
            and ("useradd" in target or "useradd" in target.lower())
        ),
        "network-ports-storage-and-health": (
            all_present(
                local,
                (
                    "EXPOSE 5030 2234",
                    "SLSKR_HTTP_BIND=0.0.0.0:5030",
                    "SLSKR_STATE_DIR=/var/lib/slskr",
                    "HEALTHCHECK",
                    "http://localhost:5030/health",
                ),
            )
            and all_present(target, ("HEALTHCHECK", "SLSKD_HTTP_PORT", "SLSKD_APP_DIR"))
        ),
        "failure-rollback-uninstall-and-logs": False,
    }


def ci_cases(local_root: Path, target_root: Path) -> dict[str, bool]:
    local = text(local_root / ".github/workflows/ci.yml")
    target = text(target_root / ".github/workflows/ci.yml")
    return {
        "build-render-and-artifact-contents": (
            all_present(
                local,
                (
                    "cargo test --workspace",
                    "npm --prefix web run build",
                    "node web/scripts/verify-build-output.mjs",
                    "node web/scripts/smoke-subpath-build.mjs",
                ),
            )
            and "upload-artifact" in target
            and (
                "./bin/build --web-only" in target
                or "npm run build" in target
                or "Release Gate" in target
            )
            and (
                "./bin/build --dotnet-only" in target
                or "bin/publish" in target
                or "dotnet publish" in target
            )
        ),
        "fresh-install-and-upgrade": False,
        "start-stop-signal-and-restart": False,
        "configuration-user-permissions-and-secrets": False,
        "network-ports-storage-and-health": False,
        "failure-rollback-uninstall-and-logs": False,
    }


def rpm_cases(local_root: Path, target_root: Path) -> dict[str, bool]:
    local = text(local_root / "packaging/rpm/slskr.spec")
    target = text(target_root / "packaging/rpm/slskdn.spec")
    return {
        "build-render-and-artifact-contents": (
            all_present(
                local,
                (
                    "%install",
                    "install -Dm755 slskr",
                    "%{_unitdir}/slskr.service",
                    "%{_sysusersdir}/slskr.conf",
                    "%{_tmpfilesdir}/slskr.conf",
                    "%{_datadir}/slskr/web/build",
                    "%files",
                ),
            )
            and all_present(target, ("%install", "%{_unitdir}", "%{_sysusersdir}", "%files"))
        ),
        "fresh-install-and-upgrade": (
            all_present(
                local,
                (
                    "%pre",
                    "%post",
                    "%config(noreplace)",
                    "%tmpfiles_create",
                    "%systemd_post",
                ),
            )
            and all_present(target, ("%pre", "%post", "%config(noreplace)", "%systemd_post"))
        ),
        "start-stop-signal-and-restart": (
            all_present(
                local,
                ("%systemd_post", "%systemd_preun", "%systemd_postun_with_restart"),
            )
            and all_present(
                target,
                ("%systemd_post", "%systemd_preun", "%systemd_postun_with_restart"),
            )
        ),
        "configuration-user-permissions-and-secrets": (
            all_present(
                local,
                (
                    "%{_sysusersdir}/slskr.conf",
                    "%{_tmpfilesdir}/slskr.conf",
                    "%config(noreplace) %{_sysconfdir}/slskr/config.toml",
                ),
            )
            and all_present(target, ("%{_sysusersdir}", "%{_tmpfilesdir}", "%config(noreplace)"))
        ),
        "network-ports-storage-and-health": False,
        "failure-rollback-uninstall-and-logs": (
            all_present(
                local,
                ("%preun", "%postun", "%systemd_postun_with_restart"),
            )
            and all_present(target, ("%preun", "%postun", "%systemd_postun_with_restart"))
        ),
    }


def winget_cases(local_root: Path, target_root: Path) -> dict[str, bool]:
    local_version = text(local_root / "packaging/winget/snapetech.slskr.yaml")
    local_locale = text(local_root / "packaging/winget/snapetech.slskr.locale.en-US.yaml")
    local_installer = text(local_root / "packaging/winget/snapetech.slskr.installer.yaml")
    target_version = text(target_root / "packaging/winget/snapetech.slskdn.yaml")
    target_locale = text(target_root / "packaging/winget/snapetech.slskdn.locale.en-US.yaml")
    target_installer = text(target_root / "packaging/winget/snapetech.slskdn.installer.yaml")

    local_version_value = re.search(r"^PackageVersion:\s*(.+)$", local_version, re.MULTILINE)
    local_installer_hash = re.search(r"^\s*InstallerSha256:\s*([0-9A-Fa-f]{64})$", local_installer, re.MULTILINE)
    local_identifier = "PackageIdentifier: snapetech.slskr" in local_version
    local_consistent = (
        local_version_value is not None
        and f"PackageVersion: {local_version_value.group(1)}" in local_locale
        and f"PackageVersion: {local_version_value.group(1)}" in local_installer
        and local_installer_hash is not None
        and local_identifier
        and "InstallerType: zip" in local_installer
        and "NestedInstallerType: portable" in local_installer
        and "PortableCommandAlias: slskr" in local_installer
        and "InstallerUrl: https://github.com/snapetech/slskr/releases/" in local_installer
    )
    target_consistent = (
        "PackageIdentifier: snapetech.slskdn" in target_version
        and "PackageIdentifier: snapetech.slskdn" in target_locale
        and "PackageIdentifier: snapetech.slskdn" in target_installer
        and re.search(r"^\s*InstallerSha256:\s*[0-9A-Fa-f]{64}$", target_installer, re.MULTILINE)
        is not None
    )
    return {
        "build-render-and-artifact-contents": local_consistent and target_consistent,
        "fresh-install-and-upgrade": (
            local_consistent
            and target_consistent
            and "ManifestType: version" in local_version
            and "ManifestType: installer" in local_installer
            and "NestedInstallerType: portable" in target_installer
        ),
        "start-stop-signal-and-restart": False,
        "configuration-user-permissions-and-secrets": False,
        "network-ports-storage-and-health": False,
        "failure-rollback-uninstall-and-logs": False,
    }


def chocolatey_cases(local_root: Path, target_root: Path) -> dict[str, bool]:
    local_spec = text(local_root / "packaging/chocolatey/slskr.nuspec")
    target_spec = text(target_root / "packaging/chocolatey/slskdn.nuspec")
    local_install = text(local_root / "packaging/chocolatey/tools/chocolateyinstall.ps1")
    target_install = text(target_root / "packaging/chocolatey/tools/chocolateyinstall.ps1")
    package_contract = ("<package", "<metadata>", "<id>", "<version>", "<files>")
    install_contract = ("Install-ChocolateyZipPackage", "-Checksum", "-ChecksumType 'sha256'")
    return {
        "build-render-and-artifact-contents": all_present(local_spec, package_contract)
        and all_present(target_spec, package_contract)
        and all_present(local_install, install_contract)
        and all_present(target_install, install_contract),
        "fresh-install-and-upgrade": all_present(local_install, ("Install-ChocolateyZipPackage",))
        and all_present(target_install, ("Install-ChocolateyZipPackage",)),
        "start-stop-signal-and-restart": False,
        "configuration-user-permissions-and-secrets": False,
        "network-ports-storage-and-health": False,
        "failure-rollback-uninstall-and-logs": False,
    }


def docker_cases(local_root: Path, target_root: Path) -> dict[str, bool]:
    local_paths = [local_root / "Dockerfile", local_root / "packaging/docker/release.Dockerfile"]
    target_paths = [
        target_root / "packaging/docker/Dockerfile.all-tools",
        target_root / "packaging/docker/Dockerfile.experimental-media",
        target_root / "packaging/docker/slskdn-container-start",
    ]
    if not all(path.is_file() for path in local_paths + target_paths):
        return {case: False for case in CASES}
    local = "\n".join(text(path) for path in local_paths)
    target = "\n".join(text(path) for path in target_paths)
    return {
        "build-render-and-artifact-contents": all_present(
            local, ("FROM ", "COPY --from=", "ENTRYPOINT", "/usr/local/bin/slskr")
        ) and all_present(target, ("FROM ", "COPY", "exec")),
        "fresh-install-and-upgrade": False,
        "start-stop-signal-and-restart": all_present(local, ("ENTRYPOINT", "CMD"))
        and all_present(target, ("exec", "SLSKD_APP_DIR")),
        "configuration-user-permissions-and-secrets": all_present(
            local, ("USER slskr", "SLSKR_STATE_DIR", "/etc/slskr")
        ) and all_present(target, ("SLSKD_APP_DIR", "umask", "config")),
        "network-ports-storage-and-health": all_present(
            local, ("EXPOSE 5030 2234", "HEALTHCHECK", "SLSKR_HTTP_BIND")
        ) and all_present(target, ("SLSKD_HTTP_PORT", "SLSKD_APP_DIR")),
        "failure-rollback-uninstall-and-logs": False,
    }


def systemd_cases(local_root: Path, target_root: Path) -> dict[str, bool]:
    local_paths = sorted((local_root / "etc/systemd").glob("*.service"))
    target_path = target_root / "etc/systemd/slskd-hardened.service"
    if not local_paths or not target_path.is_file():
        return {case: False for case in CASES}
    local = "\n".join(text(path) for path in local_paths)
    target = text(target_path)
    core = ("[Unit]", "[Service]", "ExecStart=", "Restart=on-failure")
    return {
        "build-render-and-artifact-contents": all_present(local, core)
        and all_present(target, core),
        "fresh-install-and-upgrade": False,
        "start-stop-signal-and-restart": all_present(
            local, ("ExecStart=", "Restart=on-failure", "WantedBy=multi-user.target")
        )
        and all_present(target, ("ExecStart=", "Restart=on-failure", "WantedBy=multi-user.target")),
        "configuration-user-permissions-and-secrets": all_present(
            local, ("User=", "Group=", "Environment=", "ReadWritePaths=")
        )
        and all_present(target, ("User=", "Group=", "Environment=", "ReadWritePaths=")),
        "network-ports-storage-and-health": False,
        "failure-rollback-uninstall-and-logs": all_present(
            local, ("Restart=on-failure", "PrivateTmp=")
        )
        and all_present(target, ("Restart=on-failure", "PrivateTmp=")),
    }


def release_installer_cases(family: str, local_root: Path, target_root: Path) -> dict[str, bool]:
    local_path = local_root / (
        "packaging/linux/install-from-release.sh"
        if family == "packaging-linux"
        else "packaging/proxmox-lxc/setup-inside-ct.sh"
    )
    target_path = target_root / (
        "packaging/linux/install-from-release.sh"
        if family == "packaging-linux"
        else "packaging/proxmox-lxc/setup-inside-ct.sh"
    )
    if not local_path.is_file() or not target_path.is_file():
        return {case: False for case in CASES}
    local = text(local_path)
    target = text(target_path)
    return {
        "build-render-and-artifact-contents": all_present(
            local, ("set -e", "sha256sum", "systemctl")
        )
        and all_present(target, ("set -e", "sha256sum", "systemctl"))
        and ("tar" in local or "unzip" in local)
        and ("tar" in target or "unzip" in target),
        "fresh-install-and-upgrade": all_present(
            local, ("mkdir -p", "useradd", "CONFIG_DIR")
        )
        and all_present(target, ("mkdir -p", "useradd", "CONFIG_DIR")),
        "start-stop-signal-and-restart": all_present(
            local, ("systemctl stop", "Restart=on-failure", "daemon-reload")
        )
        and all_present(target, ("systemctl stop", "Restart=on-failure", "daemon-reload")),
        "configuration-user-permissions-and-secrets": all_present(
            local, ("CONFIG_FILE", "chown", "5030")
        )
        and all_present(target, ("CONFIG_FILE", "chown", "5030")),
        "network-ports-storage-and-health": all_present(
            local, ("5030", "network-online.target", "DATA_DIR")
        )
        and all_present(target, ("5030", "network-online.target", "DATA_DIR")),
        "failure-rollback-uninstall-and-logs": all_present(
            local, ("set -e", "sha256sum", "exit 1")
        )
        and all_present(target, ("set -e", "sha256sum", "exit 1")),
    }


def flatpak_cases(local_root: Path, target_root: Path) -> dict[str, bool]:
    local_path = local_root / "packaging/flatpak/io.github.slskd.slskr.yml"
    target_path = target_root / "packaging/flatpak/io.github.slskd.slskdn.yml"
    if not local_path.is_file() or not target_path.is_file():
        return {case: False for case in CASES}
    local = text(local_path)
    target = text(target_path)
    core = ("runtime:", "sdk:", "finish-args:", "modules:", "sha256:")
    return {
        "build-render-and-artifact-contents": all_present(local, core)
        and all_present(target, core),
        "fresh-install-and-upgrade": all_present(local, ("build-commands:", "sources:"))
        and all_present(target, ("build-commands:", "sources:")),
        "start-stop-signal-and-restart": False,
        "configuration-user-permissions-and-secrets": all_present(
            local, (".config", "mkdir -p", "exec")
        )
        and all_present(target, (".config", "mkdir -p", "exec")),
        "network-ports-storage-and-health": all_present(local, ("--share=network", "--filesystem="))
        and all_present(target, ("--share=network", "--filesystem=")),
        "failure-rollback-uninstall-and-logs": False,
    }


def snap_cases(local_root: Path, target_root: Path) -> dict[str, bool]:
    local_path = local_root / "packaging/snap/snapcraft.yaml"
    target_path = target_root / "packaging/snap/snapcraft.yaml"
    if not local_path.is_file() or not target_path.is_file():
        return {case: False for case in CASES}
    local = text(local_path)
    target = text(target_path)
    return {
        "build-render-and-artifact-contents": all_present(
            local, ("name:", "base:", "parts:", "source-checksum:")
        )
        and all_present(target, ("name:", "base:", "parts:", "source-checksum:")),
        "fresh-install-and-upgrade": all_present(local, ("confinement:", "architectures:"))
        and all_present(target, ("confinement:", "architectures:")),
        "start-stop-signal-and-restart": all_present(local, ("daemon: simple", "command:"))
        and all_present(target, ("daemon: simple", "command:")),
        "configuration-user-permissions-and-secrets": all_present(
            local, ("environment:", "SNAP_USER_COMMON")
        )
        and all_present(target, ("environment:", "SNAP_USER_COMMON")),
        "network-ports-storage-and-health": all_present(local, ("network", "network-bind"))
        and all_present(target, ("network", "network-bind")),
        "failure-rollback-uninstall-and-logs": False,
    }


def chart_cases(
    local_path: Path, target_path: Path, *, controller_lifecycle: bool = False
) -> dict[str, bool]:
    if not local_path.is_dir() or not target_path.is_dir():
        return {case: False for case in CASES}
    local_paths = list(local_path.rglob("*"))
    target_paths = list(target_path.rglob("*"))
    local = "\n".join(text(path) for path in local_paths if path.is_file())
    target = "\n".join(text(path) for path in target_paths if path.is_file())
    local_probe = all_present(local, ("livenessProbe:", "readinessProbe:")) or (
        "common.lib.controller.probe" in local
    )
    target_probe = all_present(target, ("livenessProbe:", "readinessProbe:")) or (
        "common.lib.controller.probe" in target
    )
    local_upgrade = all_present(local, ("helm install", "helm upgrade"))
    target_upgrade = all_present(target, ("helm install", "helm upgrade"))
    if controller_lifecycle:
        target_upgrade = "helm install" in target and "upgrad" in target.lower()
    return {
        "build-render-and-artifact-contents": all_present(local, ("apiVersion:", "kind: Deployment", "kind: Service"))
        and all_present(target, ("apiVersion:", "kind: Deployment", "kind: Service")),
        "fresh-install-and-upgrade": local_upgrade and target_upgrade,
        "start-stop-signal-and-restart": all_present(local, ("replicas:",))
        and all_present(target, ("replicas:",))
        and local_probe
        and target_probe,
        "configuration-user-permissions-and-secrets": all_present(local, ("persistence:", "securityContext:", "runAsNonRoot"))
        and all_present(target, ("persistence:", "securityContext:", "runAsNonRoot")),
        "network-ports-storage-and-health": all_present(local, ("containerPort:", "port: 5030"))
        and all_present(target, ("containerPort:", "port: 5030"))
        and local_probe
        and target_probe,
        "failure-rollback-uninstall-and-logs": False,
    }


def helm_cases(local_root: Path, target_root: Path) -> dict[str, bool]:
    return chart_cases(local_root / "packaging/helm/slskr", target_root / "packaging/helm/slskdn")


def truenas_cases(local_root: Path, target_root: Path) -> dict[str, bool]:
    return chart_cases(
        local_root / "packaging/truenas-scale/charts/slskr",
        target_root / "packaging/truenas-scale/charts/slskdn",
        controller_lifecycle=True,
    )


def unraid_cases(local_root: Path, target_root: Path) -> dict[str, bool]:
    local_path = local_root / "packaging/unraid/slskr.xml"
    target_path = target_root / "packaging/unraid/slskdn.xml"
    if not local_path.is_file() or not target_path.is_file():
        return {case: False for case in CASES}
    local = text(local_path)
    target = text(target_path)
    core = ("<Container", "<Repository>", "<WebUI>", "<Networking>", "<Data>")
    return {
        "build-render-and-artifact-contents": all_present(local, core)
        and all_present(target, core),
        "fresh-install-and-upgrade": all_present(local, ("<Name>", "<Branch>"))
        and all_present(target, ("<Name>", "<Branch>")),
        "start-stop-signal-and-restart": all_present(local, ("--restart=unless-stopped", "<Privileged>false"))
        and all_present(target, ("--restart=unless-stopped", "<Privileged>false")),
        "configuration-user-permissions-and-secrets": all_present(local, ("<Environment>", "<Config"))
        and all_present(target, ("<Environment>", "<Config")),
        "network-ports-storage-and-health": all_present(local, ("<Port>", "<HostPort>", "<ContainerPort>"))
        and all_present(target, ("<Port>", "<HostPort>", "<ContainerPort>")),
        "failure-rollback-uninstall-and-logs": False,
    }


def synology_cases(local_root: Path, target_root: Path) -> dict[str, bool]:
    local_root = local_root / "packaging/synology-spk"
    target_root = target_root / "packaging/synology-spk"
    required = (
        "INFO",
        "build-spk.sh",
        "scripts/common",
        "scripts/preinst",
        "scripts/postinst",
        "scripts/preuninst",
        "scripts/postuninst",
        "scripts/start-stop-status",
    )
    if not all((local_root / path).is_file() for path in required) or not all(
        (target_root / path).is_file() for path in required
    ):
        return {case: False for case in CASES}
    local = "\n".join(text(local_root / path) for path in required)
    target = "\n".join(text(target_root / path) for path in required)
    return {
        "build-render-and-artifact-contents": all_present(
            local, ("package", "version", "arch", "package.tgz")
        )
        and all_present(target, ("package", "version", "arch", "package.tgz")),
        "fresh-install-and-upgrade": all(
            (local_root / path).is_file()
            for path in ("scripts/preinst", "scripts/postinst")
        )
        and all(
            (target_root / path).is_file()
            for path in ("scripts/preinst", "scripts/postinst")
        ),
        "start-stop-signal-and-restart": all_present(local, ("start", "stop", "restart", "status"))
        and all_present(target, ("start", "stop", "restart", "status")),
        "configuration-user-permissions-and-secrets": all_present(
            local, ("config", "chmod", "chown")
        )
        and all_present(target, ("config", "chmod", "chown")),
        "network-ports-storage-and-health": all_present(local, ("5030", "/var/packages", "status"))
        and all_present(target, ("5030", "/var/packages", "status")),
        "failure-rollback-uninstall-and-logs": all(
            (local_root / path).is_file()
            for path in ("scripts/preuninst", "scripts/postuninst")
        )
        and all(
            (target_root / path).is_file()
            for path in ("scripts/preuninst", "scripts/postuninst")
        )
        and all_present(local, ("backup_config", "log_step"))
        and all_present(target, ("backup_config", "log_step")),
    }


def nix_cases(local_root: Path, target_root: Path) -> dict[str, bool]:
    local_path = local_root / "flake.nix"
    target_path = target_root / "flake.nix"
    if not local_path.is_file() or not target_path.is_file():
        return {case: False for case in CASES}
    local = text(local_path)
    target = text(target_path)
    return {
        "build-render-and-artifact-contents": all_present(local, ("nixpkgs", "mkDerivation", "installPhase"))
        and all_present(target, ("nixpkgs", "mkDerivation", "installPhase")),
        "fresh-install-and-upgrade": all_present(local, ("packages =", "default ="))
        and all_present(target, ("packages =", "default =")),
        "start-stop-signal-and-restart": False,
        "configuration-user-permissions-and-secrets": all_present(local, ("makeWrapper", "bin"))
        and all_present(target, ("makeWrapper", "bin")),
        "network-ports-storage-and-health": False,
        "failure-rollback-uninstall-and-logs": False,
    }


def vpn_agent_cases(local_root: Path, target_root: Path) -> dict[str, bool]:
    local_root = local_root / "src/slskdN.VpnAgent"
    target_root = target_root / "src/slskdN.VpnAgent"
    local_required = ("install.sh", "slskr-vpn-agent")
    target_required = ("install.sh", "Program.cs")
    if not all((local_root / path).is_file() for path in local_required) or not all(
        (target_root / path).is_file() for path in target_required
    ):
        return {case: False for case in CASES}
    local_paths = [path for path in local_root.rglob("*") if path.is_file()]
    target_paths = [path for path in target_root.rglob("*") if path.is_file()]
    local = "\n".join(text(path) for path in local_paths)
    target = "\n".join(text(path) for path in target_paths)
    return {
        "build-render-and-artifact-contents": all_present(
            local, ("set -e", "install -D", "slskr-vpn-agent", "systemctl")
        )
        and all_present(target, ("set -e", "dotnet publish", "install_file", "systemctl")),
        "fresh-install-and-upgrade": all_present(
            local, ("install -d", "systemctl enable", "/var/lib/slskr-vpn")
        )
        and all_present(target, ("dotnet publish", "systemctl enable", "/var/lib/slskdN-vpn")),
        "start-stop-signal-and-restart": all_present(
            local, ("ExecStart=", "systemctl daemon-reload", "Restart=", "OnUnitActiveSec=")
        )
        and all_present(target, ("ExecStart=", "systemctl daemon-reload", "Restart=", "OnUnitActiveSec=")),
        "configuration-user-permissions-and-secrets": all_present(
            local, ("SLSKR_VPN_STATE_DIR", "ReadWritePaths=", "ProtectHome=true")
        )
        and all_present(target, ("SLSKDN_VPN_STATE_DIR", "ReadWritePaths=", "ProtectHome=true")),
        "network-ports-storage-and-health": all_present(
            local, ("/v1/portforward", "curl", "ip", "iptables")
        )
        and all_present(target, ("/v1/portforward", "curl", "ip", "iptables")),
        "failure-rollback-uninstall-and-logs": all_present(
            local, ("verify", "watchdog", "cleanup-ingress", "systemctl restart")
        )
        and all_present(target, ("verify", "watchdog", "cleanup-ingress", "systemctl", "logger")),
    }


def workflow_cases(family: str, local_root: Path, target_root: Path) -> dict[str, bool]:
    """Check artifact-producing upstream workflows against consolidated Rust workflows.

    slskr intentionally consolidates the frozen build/release workflow graph:
    ``release.yml`` owns cross-platform archives and ``release-publish.yml``
    owns downstream package publication.  The workflow-level lifecycle is
    therefore checked at the artifact family, service, and package layers
    rather than duplicated in every GitHub workflow file.
    """
    target_path = target_root / ".github/workflows" / f"{family.removeprefix('github-workflow-')}.yml"
    if not target_path.is_file():
        return {case: False for case in CASES}
    target = text(target_path)
    local_sources = []
    for relative in (
        ".github/workflows/ci.yml",
        ".github/workflows/release.yml",
        ".github/workflows/release-publish.yml",
        ".github/workflows/publish-chocolatey.yml",
    ):
        path = local_root / relative
        if path.is_file():
            local_sources.append(text(path))
    local = "\n".join(local_sources)

    if family == "github-workflow-ci":
        build = (
            all_present(local, ("cargo test --workspace", "npm --prefix web run build"))
            and "upload-artifact" in target
            and ("./bin/build --web-only" in target or "dotnet publish" in target)
        )
    elif family == "github-workflow-publish-chocolatey":
        build = "choco pack" in target and all_present(
            local, ("choco pack",)
        ) and ("packaging/chocolatey" in local or "packaging\\chocolatey" in local)
    elif family == "github-workflow-publish-winget":
        build = all_present(target, ("wingetcreate", "packaging/winget")) and all_present(
            local,
            ("wingetcreate", "packaging/winget"),
        )
    elif family == "github-workflow-release-copr":
        build = all_present(target, ("rpmbuild", "copr-cli")) and all_present(
            local,
            ("rpmbuild", "copr-cli", "packaging/rpm/slskr.spec"),
        )
    elif family == "github-workflow-release-linux":
        build = all_present(target, ("upload-artifact", "dotnet publish")) and all_present(
            local,
            ("upload-artifact", "scripts/build-release-archive.sh"),
        )
    elif family == "github-workflow-release-packages":
        build = all_present(target, ("dpkg-buildpackage", "rpmbuild")) and all_present(
            local,
            ("debuild", "rpmbuild", "packaging/debian", "packaging/rpm/slskr.spec"),
        )
    elif family == "github-workflow-release-ppa":
        build = all_present(target, ("debuild", "dput")) and all_present(
            local,
            ("debuild", "dput", "packaging/debian"),
        )
    elif family == "github-workflow-build-on-tag":
        build = all_present(target, ("upload-artifact", "dotnet publish")) and all_present(
            local,
            ("upload-artifact", "scripts/build-release-archive.sh"),
        )
    else:
        build = False
    return {
        "build-render-and-artifact-contents": build,
        "fresh-install-and-upgrade": False,
        "start-stop-signal-and-restart": False,
        "configuration-user-permissions-and-secrets": False,
        "network-ports-storage-and-health": False,
        "failure-rollback-uninstall-and-logs": False,
    }


def cases_for(family: str, local_root: Path, target_root: Path) -> dict[str, bool]:
    if family == "container-root":
        return container_cases(local_root, target_root)
    if family == "github-workflow-ci":
        return ci_cases(local_root, target_root)
    if family == "packaging-rpm":
        return rpm_cases(local_root, target_root)
    if family == "packaging-aur":
        return aur_cases(local_root, target_root)
    if family == "packaging-debian":
        return debian_cases(local_root, target_root)
    if family == "packaging-homebrew":
        return homebrew_cases(local_root, target_root)
    if family == "packaging-winget":
        return winget_cases(local_root, target_root)
    if family == "packaging-chocolatey":
        return chocolatey_cases(local_root, target_root)
    if family == "packaging-docker":
        return docker_cases(local_root, target_root)
    if family == "packaging-linux":
        return release_installer_cases(family, local_root, target_root)
    if family == "packaging-proxmox-lxc":
        return release_installer_cases(family, local_root, target_root)
    if family == "packaging-flatpak":
        return flatpak_cases(local_root, target_root)
    if family == "packaging-snap":
        return snap_cases(local_root, target_root)
    if family == "packaging-helm":
        return helm_cases(local_root, target_root)
    if family == "packaging-truenas-scale":
        return truenas_cases(local_root, target_root)
    if family == "packaging-unraid":
        return unraid_cases(local_root, target_root)
    if family == "packaging-synology-spk":
        return synology_cases(local_root, target_root)
    if family == "nix-root":
        return nix_cases(local_root, target_root)
    if family == "vpn-agent":
        return vpn_agent_cases(local_root, target_root)
    if family == "systemd-hardened":
        return systemd_cases(local_root, target_root)
    if family.startswith("github-workflow-"):
        return workflow_cases(family, local_root, target_root)
    return {case: False for case in CASES}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--slskr-root", type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument("--slskd-root", type=Path, required=True)
    parser.add_argument("--slskdn-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    local_root = args.slskr_root.resolve()
    target_roots = {
        "slskd": args.slskd_root.resolve(),
        "slskdn": args.slskdn_root.resolve(),
    }
    local_families = operator_families(local_root)
    rows = []
    for target, target_root in target_roots.items():
        target_families = operator_families(target_root)
        for family in sorted(target_families):
            results = (
                cases_for(family, local_root, target_root)
                if family in local_families
                or family.startswith("github-workflow-")
                or family == "systemd-hardened"
                else {case: False for case in CASES}
            )
            for case in CASES:
                rows.append(
                    {
                        "target": target,
                        "subject": family,
                        "case": case,
                        "pass": bool(results[case]),
                    }
                )

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(rows, indent=2) + "\n", encoding="utf-8")
    passed = sum(1 for row in rows if row["pass"])
    print(f"operator packaging audit: {passed}/{len(rows)} cases passed")


if __name__ == "__main__":
    main()
