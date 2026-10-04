class Slskr < Formula
  desc "Rust Soulseek daemon with bundled Web UI"
  homepage "https://github.com/snapetech/slskr"
  license "AGPL-3.0-only"
  version "0.2.52"

  on_macos do
    on_arm do
      url "https://github.com/snapetech/slskr/releases/download/release-v0.2.52/slskr-v0.2.52-aarch64-apple-darwin.tar.gz"
      sha256 "19a421af93cdc436bb09660976ad5bccf2e81a8e6bd9b970363f8e3e4950c15a"
    end
    on_intel do
      url "https://github.com/snapetech/slskr/releases/download/release-v0.2.52/slskr-v0.2.52-x86_64-apple-darwin.tar.gz"
      sha256 "bc87f52510b369d3d1360d9739b5bb1c76ec8d735267bab7b86a28b13cd4213c"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/snapetech/slskr/releases/download/release-v0.2.52/slskr-v0.2.52-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "34327c6d59af04edb6f0cdd3f13965cdd82f808934a1e54eb9acd29e1e969ddd"
    else
      url "https://github.com/snapetech/slskr/releases/download/release-v0.2.52/slskr-v0.2.52-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "0afd6d2ba92fcf37f8bd360979b48c5e163ad96ff388c60bd0c2b999bc5016f4"
    end
  end

  def install
    libexec.install Dir["*"]
    bin.install libexec/"slskr"
  end

  test do
    assert_match "slskr", shell_output("#{bin}/slskr version")
  end
end
