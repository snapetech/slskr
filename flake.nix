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
        version = "0.2.40";
        sources = {
          "x86_64-linux" = {
            url = "https://github.com/snapetech/slskr/releases/download/release-v0.2.40/slskr-v0.2.40-x86_64-unknown-linux-gnu.tar.gz";
            sha256 = "dda79c3694f76f1b9fa547d91310bdc7b9407c77bf33c812e3ea0485bd7dcaff";
          };
          "aarch64-linux" = {
            url = "https://github.com/snapetech/slskr/releases/download/release-v0.2.40/slskr-v0.2.40-aarch64-unknown-linux-gnu.tar.gz";
            sha256 = "405eaadb27a603890619927b9bddf1b4c1c59573c77a86b24c9095a16aae9ecc";
          };
          "x86_64-darwin" = {
            url = "https://github.com/snapetech/slskr/releases/download/release-v0.2.40/slskr-v0.2.40-x86_64-apple-darwin.tar.gz";
            sha256 = "133b437c92c2b3a4338f9306073cab9c44dc7262ea860df49762abe5810aeaf5";
          };
          "aarch64-darwin" = {
            url = "https://github.com/snapetech/slskr/releases/download/release-v0.2.40/slskr-v0.2.40-aarch64-apple-darwin.tar.gz";
            sha256 = "2c652a2ecea2581e80781ee3cd95b4bf930b298e771d7e3cdef57974f23f4c4b";
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
