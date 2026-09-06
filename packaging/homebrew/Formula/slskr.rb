class Slskr < Formula
  desc "Rust Soulseek daemon with bundled Web UI"
  homepage "https://github.com/snapetech/slskr"
  license "AGPL-3.0-only"
  version "0.2.40"

  on_macos do
    on_arm do
      url "https://github.com/snapetech/slskr/releases/download/release-v0.2.40/slskr-v0.2.40-aarch64-apple-darwin.tar.gz"
      sha256 "2c652a2ecea2581e80781ee3cd95b4bf930b298e771d7e3cdef57974f23f4c4b"
    end
    on_intel do
      url "https://github.com/snapetech/slskr/releases/download/release-v0.2.40/slskr-v0.2.40-x86_64-apple-darwin.tar.gz"
      sha256 "133b437c92c2b3a4338f9306073cab9c44dc7262ea860df49762abe5810aeaf5"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/snapetech/slskr/releases/download/release-v0.2.40/slskr-v0.2.40-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "405eaadb27a603890619927b9bddf1b4c1c59573c77a86b24c9095a16aae9ecc"
    else
      url "https://github.com/snapetech/slskr/releases/download/release-v0.2.40/slskr-v0.2.40-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "dda79c3694f76f1b9fa547d91310bdc7b9407c77bf33c812e3ea0485bd7dcaff"
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
