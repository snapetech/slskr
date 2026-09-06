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
        version = "0.2.39";
        sources = {
          "x86_64-linux" = {
            url = "https://github.com/snapetech/slskr/releases/download/release-v0.2.39/slskr-v0.2.39-x86_64-unknown-linux-gnu.tar.gz";
            sha256 = "9c34e83d7aa10b08f11a62c308528a30f428c48435c75d5d4042b1debad47185";
          };
          "aarch64-linux" = {
            url = "https://github.com/snapetech/slskr/releases/download/release-v0.2.39/slskr-v0.2.39-aarch64-unknown-linux-gnu.tar.gz";
            sha256 = "c11831e4b402d3e507ee23722a2fa46ad863daf821cc07615013beaf0ca3a76d";
          };
          "x86_64-darwin" = {
            url = "https://github.com/snapetech/slskr/releases/download/release-v0.2.39/slskr-v0.2.39-x86_64-apple-darwin.tar.gz";
            sha256 = "541a41b5a61ed3b00884fd5828c2e1eb95af159230a182d810e01a1af970dbe1";
          };
          "aarch64-darwin" = {
            url = "https://github.com/snapetech/slskr/releases/download/release-v0.2.39/slskr-v0.2.39-aarch64-apple-darwin.tar.gz";
            sha256 = "5eb7fb6fc6f33299444a1502fc1b2dd23a22d2f4de87586d90265624077b6b0f";
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
