class Amaru < Formula
  desc "A Cardano blockchain node implementation"
  homepage "https://github.com/pragma-org/amaru"
  version "10.10.20260611"
  license "Apache-2.0"

  on_macos do
    depends_on arch: :arm64

    on_arm do
      url "https://github.com/pragma-org/amaru/releases/download/v10.10.20260611/amaru-10.10.20260611-macos-aarch64.tar.gz"
      sha256 "f4605b75e6546338c3be84aae417201993db30b29e8c4fc583cbdf5784d43d10"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/pragma-org/amaru/releases/download/v10.10.20260611/amaru-10.10.20260611-linux-aarch64.tar.gz"
      sha256 "d8b9c406d224a854ec5149c5459d659874e6ed9d2d10563b2ad0efbf8d90f8e3"
    end

    on_intel do
      url "https://github.com/pragma-org/amaru/releases/download/v10.10.20260611/amaru-10.10.20260611-linux-x86_64.tar.gz"
      sha256 "ab0a12686a9cbae8371fd315ea599f05abef82c204f71d089003a6736284bc2f"
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
