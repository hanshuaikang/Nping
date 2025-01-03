class Nping < Formula
  desc "🏎 Nping mean NB Ping, A Ping Tool in Rust with Real-Time Data and Visualizations"
  homepage "https://github.com/hanshuaikang/Nping"
  url "https://github.com/hanshuaikang/Nping/releases/download/v0.2.0/Nping-x86_64-apple-darwin.zip"
  sha256 "6d2919f140a87a87a5f404eee28415485267720e41508591032257aaac07ef15"

  # for arm64
  on_arm do
    url "https://github.com/hanshuaikang/Nping/releases/download/v0.2.0/Nping-aarch64-apple-darwin.zip"
    sha256 "14d4d9a3944c5b40668ee02191335061ff5b8cec6fac4d984ab990d2273f891b"
  end

  def install
    bin.install "nping"
  end

  test do
    system "#{bin}/nping", "--version"
  end
end
