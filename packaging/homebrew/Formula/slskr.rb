class Slskr < Formula
  desc "Rust Soulseek daemon with bundled Web UI"
  homepage "https://github.com/snapetech/slskr"
  license "AGPL-3.0-only"
  version "0.2.38"

  on_macos do
    on_arm do
      url "https://github.com/snapetech/slskr/releases/download/release-v0.2.38/slskr-v0.2.38-aarch64-apple-darwin.tar.gz"
      sha256 "6224edd7aa9ec6890deeb82ba00a4e0456bb8aa491aaf870b657df5cf875a11c"
    end
    on_intel do
      url "https://github.com/snapetech/slskr/releases/download/release-v0.2.38/slskr-v0.2.38-x86_64-apple-darwin.tar.gz"
      sha256 "f35e8e4a6bb2ace9d13d87a9a6b6612e19136d30b2b9873e37e4f9c1d3ff7db9"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/snapetech/slskr/releases/download/release-v0.2.38/slskr-v0.2.38-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "1e1c09c61674afaba716c2cf15cc9afcbf7462f845b16d91e91a91650f366df1"
    else
      url "https://github.com/snapetech/slskr/releases/download/release-v0.2.38/slskr-v0.2.38-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "254ef1125c7e15f015ec0ead8b563e8755b8be16b74f9261b8592c5202fb95f0"
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
