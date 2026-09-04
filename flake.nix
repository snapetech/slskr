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
        version = "0.2.38";
        sources = {
          "x86_64-linux" = {
            url = "https://github.com/snapetech/slskr/releases/download/release-v0.2.38/slskr-v0.2.38-x86_64-unknown-linux-gnu.tar.gz";
            sha256 = "254ef1125c7e15f015ec0ead8b563e8755b8be16b74f9261b8592c5202fb95f0";
          };
          "aarch64-linux" = {
            url = "https://github.com/snapetech/slskr/releases/download/release-v0.2.38/slskr-v0.2.38-aarch64-unknown-linux-gnu.tar.gz";
            sha256 = "1e1c09c61674afaba716c2cf15cc9afcbf7462f845b16d91e91a91650f366df1";
          };
          "x86_64-darwin" = {
            url = "https://github.com/snapetech/slskr/releases/download/release-v0.2.38/slskr-v0.2.38-x86_64-apple-darwin.tar.gz";
            sha256 = "f35e8e4a6bb2ace9d13d87a9a6b6612e19136d30b2b9873e37e4f9c1d3ff7db9";
          };
          "aarch64-darwin" = {
            url = "https://github.com/snapetech/slskr/releases/download/release-v0.2.38/slskr-v0.2.38-aarch64-apple-darwin.tar.gz";
            sha256 = "6224edd7aa9ec6890deeb82ba00a4e0456bb8aa491aaf870b657df5cf875a11c";
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
