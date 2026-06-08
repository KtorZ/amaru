class Amaru < Formula
  desc "A Cardano blockchain node implementation"
  homepage "https://github.com/pragma-org/amaru"
  version "10.10.20260607"
  license "Apache-2.0"

  on_macos do
    depends_on arch: :arm64

    on_arm do
      url "https://github.com/KtorZ/amaru/releases/download/v10.10.20260607/amaru-10.10.20260607-macos-aarch64.tar.gz"
      sha256 "7acafc6900075f245a48ad40560fee84f9a7beac9d86674f982964430afc77f1"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/KtorZ/amaru/releases/download/v10.10.20260607/amaru-10.10.20260607-linux-aarch64.tar.gz"
      sha256 "06490299dc119be38ca4e8d8e48b5706f2421b638fcf43eeb04009601504d29b"
    end

    on_intel do
      url "https://github.com/KtorZ/amaru/releases/download/v10.10.20260607/amaru-10.10.20260607-linux-x86_64.tar.gz"
      sha256 "c080735877eb66870803d47e9d1d138158667af8b41675a012ed73c76a0d9718"
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
