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
        version = "0.2.41";
        sources = {
          "x86_64-linux" = {
            url = "https://github.com/snapetech/slskr/releases/download/release-v0.2.41/slskr-v0.2.41-x86_64-unknown-linux-gnu.tar.gz";
            sha256 = "0c782ada01d2bf99755ed406fbabae574eafa1b1148fbb377b3642d2178e688f";
          };
          "aarch64-linux" = {
            url = "https://github.com/snapetech/slskr/releases/download/release-v0.2.41/slskr-v0.2.41-aarch64-unknown-linux-gnu.tar.gz";
            sha256 = "6e92864e6aaaa6537da49f1ba0f6852644a5b53fa0ad53f5890273f7155f7121";
          };
          "x86_64-darwin" = {
            url = "https://github.com/snapetech/slskr/releases/download/release-v0.2.41/slskr-v0.2.41-x86_64-apple-darwin.tar.gz";
            sha256 = "562cbe042ce457512b5f8b889acc829bd0b04ec03001217fc9cd194e4f082df5";
          };
          "aarch64-darwin" = {
            url = "https://github.com/snapetech/slskr/releases/download/release-v0.2.41/slskr-v0.2.41-aarch64-apple-darwin.tar.gz";
            sha256 = "61fb1dd64819c5f924c626e57c318aba1882a6915510123771f2707db8cb19c0";
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
