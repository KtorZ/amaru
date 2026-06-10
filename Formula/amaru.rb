class Amaru < Formula
  desc "A Cardano blockchain node implementation"
  homepage "https://github.com/pragma-org/amaru"
  version "10.10.20260609"
  license "Apache-2.0"

  on_macos do
    depends_on arch: :arm64

    on_arm do
      url "https://github.com/KtorZ/amaru/releases/download/v10.10.20260609/amaru-10.10.20260609-macos-aarch64.tar.gz"
      sha256 "bd053ecb95635f4b4b5339e2e565987d8413e9ab86bf8054698414dacf31ef4a"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/KtorZ/amaru/releases/download/v10.10.20260609/amaru-10.10.20260609-linux-aarch64.tar.gz"
      sha256 "813b5ed1c2dc4d2a46014a166df5d41b369e5fcc4832d80976af1a58dbdf2ee4"
    end

    on_intel do
      url "https://github.com/KtorZ/amaru/releases/download/v10.10.20260609/amaru-10.10.20260609-linux-x86_64.tar.gz"
      sha256 "a45645c018cf6272c0db2212b93ff54c03f5a98d162c31e0bc54b915ab950e8e"
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
