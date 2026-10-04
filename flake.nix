{
  description = "slskR Rust Soulseek daemon with bundled Web UI";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        version = "0.2.52";
        sources = {
          "x86_64-linux" = {
            url = "https://github.com/snapetech/slskr/releases/download/release-v0.2.52/slskr-v0.2.52-x86_64-unknown-linux-gnu.tar.gz";
            sha256 = "0afd6d2ba92fcf37f8bd360979b48c5e163ad96ff388c60bd0c2b999bc5016f4";
          };
          "aarch64-linux" = {
            url = "https://github.com/snapetech/slskr/releases/download/release-v0.2.52/slskr-v0.2.52-aarch64-unknown-linux-gnu.tar.gz";
            sha256 = "34327c6d59af04edb6f0cdd3f13965cdd82f808934a1e54eb9acd29e1e969ddd";
          };
          "x86_64-darwin" = {
            url = "https://github.com/snapetech/slskr/releases/download/release-v0.2.52/slskr-v0.2.52-x86_64-apple-darwin.tar.gz";
            sha256 = "bc87f52510b369d3d1360d9739b5bb1c76ec8d735267bab7b86a28b13cd4213c";
          };
          "aarch64-darwin" = {
            url = "https://github.com/snapetech/slskr/releases/download/release-v0.2.52/slskr-v0.2.52-aarch64-apple-darwin.tar.gz";
            sha256 = "19a421af93cdc436bb09660976ad5bccf2e81a8e6bd9b970363f8e3e4950c15a";
          };
        };
        mkSlskr = { pname, version, sources }:
          pkgs.stdenv.mkDerivation {
            inherit pname version;
            src = pkgs.fetchurl (sources.${system});
            nativeBuildInputs = [ pkgs.makeWrapper ];
            unpackPhase = "tar xzf $src";
            installPhase = ''
              mkdir -p $out/libexec/${pname} $out/bin
              cp -r . $out/libexec/${pname}/
              chmod +x $out/libexec/${pname}/slskr
              makeWrapper $out/libexec/${pname}/slskr $out/bin/slskr
            '';
          };
      in {
        packages = {
          default = mkSlskr {
            pname = "slskr";
            inherit version sources;
          };
        };
      }
    );
}
