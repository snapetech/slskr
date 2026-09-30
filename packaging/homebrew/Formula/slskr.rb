class Slskr < Formula
  desc "Rust Soulseek daemon with bundled Web UI"
  homepage "https://github.com/snapetech/slskr"
  license "AGPL-3.0-only"
  version "0.2.41"

  on_macos do
    on_arm do
      url "https://github.com/snapetech/slskr/releases/download/release-v0.2.41/slskr-v0.2.41-aarch64-apple-darwin.tar.gz"
      sha256 "61fb1dd64819c5f924c626e57c318aba1882a6915510123771f2707db8cb19c0"
    end
    on_intel do
      url "https://github.com/snapetech/slskr/releases/download/release-v0.2.41/slskr-v0.2.41-x86_64-apple-darwin.tar.gz"
      sha256 "562cbe042ce457512b5f8b889acc829bd0b04ec03001217fc9cd194e4f082df5"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/snapetech/slskr/releases/download/release-v0.2.41/slskr-v0.2.41-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "6e92864e6aaaa6537da49f1ba0f6852644a5b53fa0ad53f5890273f7155f7121"
    else
      url "https://github.com/snapetech/slskr/releases/download/release-v0.2.41/slskr-v0.2.41-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "0c782ada01d2bf99755ed406fbabae574eafa1b1148fbb377b3642d2178e688f"
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
