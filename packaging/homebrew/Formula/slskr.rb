class Slskr < Formula
  desc "Rust Soulseek daemon with bundled Web UI"
  homepage "https://github.com/snapetech/slskr"
  license "AGPL-3.0-only"
  version "0.2.39"

  on_macos do
    on_arm do
      url "https://github.com/snapetech/slskr/releases/download/release-v0.2.39/slskr-v0.2.39-aarch64-apple-darwin.tar.gz"
      sha256 "5eb7fb6fc6f33299444a1502fc1b2dd23a22d2f4de87586d90265624077b6b0f"
    end
    on_intel do
      url "https://github.com/snapetech/slskr/releases/download/release-v0.2.39/slskr-v0.2.39-x86_64-apple-darwin.tar.gz"
      sha256 "541a41b5a61ed3b00884fd5828c2e1eb95af159230a182d810e01a1af970dbe1"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/snapetech/slskr/releases/download/release-v0.2.39/slskr-v0.2.39-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "c11831e4b402d3e507ee23722a2fa46ad863daf821cc07615013beaf0ca3a76d"
    else
      url "https://github.com/snapetech/slskr/releases/download/release-v0.2.39/slskr-v0.2.39-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "9c34e83d7aa10b08f11a62c308528a30f428c48435c75d5d4042b1debad47185"
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
