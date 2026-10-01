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
        version = "0.2.42";
        sources = {
          "x86_64-linux" = {
            url = "https://github.com/snapetech/slskr/releases/download/release-v0.2.42/slskr-v0.2.42-x86_64-unknown-linux-gnu.tar.gz";
            sha256 = "430106715561b36b074d67e8d951db6fdb03e05c77a59a291b0a05c418f342c8";
          };
          "aarch64-linux" = {
            url = "https://github.com/snapetech/slskr/releases/download/release-v0.2.42/slskr-v0.2.42-aarch64-unknown-linux-gnu.tar.gz";
            sha256 = "a2c4ab90476f5bce9c6bb6b7402623935bfe1839cbd260fb79e2aa506bf51dfb";
          };
          "x86_64-darwin" = {
            url = "https://github.com/snapetech/slskr/releases/download/release-v0.2.42/slskr-v0.2.42-x86_64-apple-darwin.tar.gz";
            sha256 = "362dbf9362e198413815ea8a24adcda17e113ed01b7c2b18965773786c89af7c";
          };
          "aarch64-darwin" = {
            url = "https://github.com/snapetech/slskr/releases/download/release-v0.2.42/slskr-v0.2.42-aarch64-apple-darwin.tar.gz";
            sha256 = "bab052efe8f285001255bd6a728ca8d66170e504f446aec27336cab65d8b8ebe";
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
