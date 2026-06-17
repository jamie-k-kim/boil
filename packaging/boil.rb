class Boil < Formula
  desc "Architectural Intelligence Platform"
  homepage "https://github.com/jamie-k-kim/boil"
  url "https://github.com/jamie-k-kim/boil/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "add_sha256_here_when_published" # Replace with actual SHA256 of release tarball

  head "https://github.com/jamie-k-kim/boil.git", branch: "main"

  depends_on "rust" => :build
  depends_on "onnxruntime"

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    system "#{bin}/boil", "--version"
  end
end
