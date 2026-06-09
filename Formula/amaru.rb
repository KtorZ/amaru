class Amaru < Formula
  desc "A Cardano blockchain node implementation"
  homepage "https://github.com/pragma-org/amaru"
  version "10.10.20260609"
  license "Apache-2.0"

  on_macos do
    depends_on arch: :arm64

    on_arm do
      url "https://github.com/KtorZ/amaru/releases/download/v10.10.20260609/amaru-10.10.20260609-macos-aarch64.tar.gz"
      sha256 "873096bcd9e92a9d9a7d7ef469dcf929794ef6dabfb1d2ef308fff9e156ff9ee"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/KtorZ/amaru/releases/download/v10.10.20260609/amaru-10.10.20260609-linux-aarch64.tar.gz"
      sha256 "f3024787ad64d4411d2cfc045f28d77ea069a9988ca5f4826253ebbcae9386e8"
    end

    on_intel do
      url "https://github.com/KtorZ/amaru/releases/download/v10.10.20260609/amaru-10.10.20260609-linux-x86_64.tar.gz"
      sha256 "cfe0d7683168e12f15f4649291c9be84b7675f3c9e6c27e566ce3da503b80e83"
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
