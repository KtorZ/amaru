class Amaru < Formula
  desc "A Cardano blockchain node implementation"
  homepage "https://github.com/pragma-org/amaru"
  version "10.10.20260609"
  license "Apache-2.0"

  on_macos do
    depends_on arch: :arm64

    on_arm do
      url "https://github.com/KtorZ/amaru/releases/download/v10.10.20260609/amaru-10.10.20260609-macos-aarch64.tar.gz"
      sha256 "fe67554d7947124fee403151a09ceea82c17a6669cfe3bcd50930bb25a1979d1"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/KtorZ/amaru/releases/download/v10.10.20260609/amaru-10.10.20260609-linux-aarch64.tar.gz"
      sha256 "10e717863fba87ad94b16e32c768f6aa2a5c960c3f8426c6fd8c244597d2ef4e"
    end

    on_intel do
      url "https://github.com/KtorZ/amaru/releases/download/v10.10.20260609/amaru-10.10.20260609-linux-x86_64.tar.gz"
      sha256 "118ee30af11a366ecb4451c1a894d1bc443e42d10cad4621ce68643836eeda66"
    end
  end

  def install
    root = if File.exist?("bin/amaru")
      Pathname.pwd
    else
      candidate = Dir["*/bin/amaru"].find { |entry| File.file?(entry) }
      candidate.nil? ? nil : Pathname.new(candidate).dirname.dirname
    end

    odie "expected extracted Amaru archive contents" if root.nil?

    bin.install root/"bin/amaru"
    man1.install root/"share/man/man1/amaru.1"
    bash_completion.install root/"share/bash-completion/completions/amaru"
    zsh_completion.install root/"share/zsh/site-functions/_amaru"
    fish_completion.install root/"share/fish/vendor_completions.d/amaru.fish"

    docs = root/"share/doc/amaru"
    if docs.directory?
      Dir[docs/"*"].sort.each do |path|
        pkgshare.install path
      end
    end
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/amaru --version")
  end
end
