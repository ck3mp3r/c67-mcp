class C67Mcp < Formula
  desc "c67-mcp - Model Context Protocol (MCP) Server for Context7"
  homepage "https://github.com/ck3mp3r/c67-mcp"
  version "0.2.2"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/c67-mcp/releases/download/v0.2.2/c67-mcp-0.2.2-aarch64-darwin.tgz"
      sha256 "ce88144e567e2661c87a5f2369b3e3e9550582dbf60c9a38de52459e85d51ee9"
    else
      odie "Intel Macs are no longer supported. Please use an Apple Silicon Mac."
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/c67-mcp/releases/download/v0.2.2/c67-mcp-0.2.2-x86_64-linux.tgz"
      sha256 "5c91557fca6383eb33f1bf0c770b0e56c43b5cf16923a6bdfe1ac209076ae557"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/c67-mcp/releases/download/v0.2.2/c67-mcp-0.2.2-aarch64-linux.tgz"
      sha256 "abcab4b589450c6d6aa53d0dbb2a41ab16dab60e374024ad254a26af43457c0d"
    end
  end

  def install
    bin.install "c67-mcp"
  end
end
