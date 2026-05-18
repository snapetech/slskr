class Slskr < Formula
  desc "Rust Soulseek daemon with bundled Web UI"
  homepage "https://github.com/snapetech/slskr"
  license "AGPL-3.0-only"
  version "0.2.15"

  on_macos do
    on_arm do
      url "https://github.com/snapetech/slskr/releases/download/release-v0.2.15/slskr-v0.2.15-aarch64-apple-darwin.tar.gz"
      sha256 "0756e01fc99c41151aae20a04ab0f7f94235c42435b3dd43b5e23a383934f486"
    end
    on_intel do
      url "https://github.com/snapetech/slskr/releases/download/release-v0.2.15/slskr-v0.2.15-x86_64-apple-darwin.tar.gz"
      sha256 "c4b4156bd3278454e53e85de21ac7a6ed206a2f90ea7526680050b07de993a71"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/snapetech/slskr/releases/download/release-v0.2.15/slskr-v0.2.15-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "021a57e4f74b6924ce400368dc3976b213b24b17bea8d1db24fbb6b212f1f661"
    else
      url "https://github.com/snapetech/slskr/releases/download/release-v0.2.15/slskr-v0.2.15-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "719ed6b26b34ec51ac45370cef64ab8dd5cf85b1f56d18f8ba38b126296a814e"
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
