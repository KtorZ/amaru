class Amaru < Formula
  desc "A Cardano blockchain node implementation"
  homepage "https://github.com/pragma-org/amaru"
  version "10.10.20260607"
  license "Apache-2.0"

  on_macos do
    depends_on arch: :arm64

    on_arm do
      url "https://github.com/KtorZ/amaru/releases/download/v10.10.20260607/amaru-10.10.20260607-macos-aarch64.tar.gz"
      sha256 "012b58766cd78a1b2d6e1e9034e08d3181985e4a0b887df37fa65c6f9b9f466c"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/KtorZ/amaru/releases/download/v10.10.20260607/amaru-10.10.20260607-linux-aarch64.tar.gz"
      sha256 "d81b4e201127822a77331ae90e32448ce6da451eb3d30b15d8147931b4947c27"
    end

    on_intel do
      url "https://github.com/KtorZ/amaru/releases/download/v10.10.20260607/amaru-10.10.20260607-linux-x86_64.tar.gz"
      sha256 "d29517a94036c6fe06d076fd9316a65081062b3d5dc3ad09fb87b8fbb66fd254"
    end
  end

  def install
    root = Dir["amaru-*"].find { |entry| File.directory?(entry) }
    odie "expected a single extracted Amaru archive directory" if root.nil?

    bin.install "#{root}/bin/amaru"
    man1.install "#{root}/share/man/man1/amaru.1"
    bash_completion.install "#{root}/share/bash-completion/completions/amaru"
    zsh_completion.install "#{root}/share/zsh/site-functions/_amaru"
    fish_completion.install "#{root}/share/fish/vendor_completions.d/amaru.fish"

    %w[LICENSE README.md CHANGELOG.md].each do |file|
      path = "#{root}/#{file}"
      pkgshare.install path if File.exist?(path)
    end
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/amaru --version")
  end
end
