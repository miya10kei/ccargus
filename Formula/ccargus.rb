class Ccargus < Formula
  desc "A TUI application for managing multiple Claude Code worktree sessions"
  homepage "https://github.com/miya10kei/ccargus"
  version "0.0.3"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/miya10kei/ccargus/releases/download/v0.0.3/ccargus-v0.0.3-darwin-arm64.tar.gz"
      sha256 "80f34b10ef11ac64a35f914bfa616e647099689dad119f4fcb2dd88a3edea518"
    end
    on_intel do
      url "https://github.com/miya10kei/ccargus/releases/download/v0.0.3/ccargus-v0.0.3-darwin-x64.tar.gz"
      sha256 "71e0237ecc97e6b53de5ccd0324021d4bfee0010a8a80ece3a594936a883c62a"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/miya10kei/ccargus/releases/download/v0.0.3/ccargus-v0.0.3-linux-x64.tar.gz"
      sha256 "3a7b22759143b6d8d2732d3e49fb186ff404f211aadc397a832d2c050a846e98"
    end
  end

  def install
    bin.install "ccargus"
    bin.install "ccargus-notify"
  end

  test do
    assert_match "ccargus #{version}", shell_output("#{bin}/ccargus --version")
  end
end
