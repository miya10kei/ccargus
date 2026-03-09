class Ccargus < Formula
  desc "A TUI application for managing multiple Claude Code worktree sessions"
  homepage "https://github.com/miya10kei/ccargus"
  version "VERSION"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/miya10kei/ccargus/releases/download/vVERSION/ccargus-vVERSION-aarch64-apple-darwin.tar.gz"
      sha256 "SHA256"
    end
    on_intel do
      url "https://github.com/miya10kei/ccargus/releases/download/vVERSION/ccargus-vVERSION-x86_64-apple-darwin.tar.gz"
      sha256 "SHA256"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/miya10kei/ccargus/releases/download/vVERSION/ccargus-vVERSION-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "SHA256"
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
