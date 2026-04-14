class Ccargus < Formula
  desc "A TUI application for managing multiple Claude Code worktree sessions"
  homepage "https://github.com/miya10kei/ccargus"
  version "0.0.4"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/miya10kei/ccargus/releases/download/v0.0.4/ccargus-v0.0.4-darwin-arm64.tar.gz"
      sha256 "87d5c768effef7a19cfcbcd4df744e0aed8ce3023af9ec1ad517d78da5335d71"
    end
    on_intel do
      url "https://github.com/miya10kei/ccargus/releases/download/v0.0.4/ccargus-v0.0.4-darwin-x64.tar.gz"
      sha256 "de3d7e7a6041bb5d0f12ae44b571f87f98bd48496aa249ea8365abc42d662ef7"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/miya10kei/ccargus/releases/download/v0.0.4/ccargus-v0.0.4-linux-arm64.tar.gz"
      sha256 "ed5028f1c5df575c2629912c26c7df37c6e3b790ab5f73fa5ab0a0d2e7c0d5bc"
    end
    on_intel do
      url "https://github.com/miya10kei/ccargus/releases/download/v0.0.4/ccargus-v0.0.4-linux-x64.tar.gz"
      sha256 "e9717a9223101fac27324e0e6dff9a9398d97538f3af4d25b0e0e770edf6d2d4"
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
