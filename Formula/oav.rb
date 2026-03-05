class Oav < Formula
  desc "OpenAPI Validator CLI for linting, generating, and compiling OpenAPI specs locally"
  homepage "https://github.com/entur/openapi-validator-cli"
  version "0.6.0"
  license "EUPL-1.2"

  # Update version, urls, and sha256 values for each release.

  on_macos do
    on_intel do
      url "https://github.com/entur/openapi-validator-cli/releases/download/v#{version}/oav-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "ca9f27387e8d37e65d7cd42725df94e27dce03f5d1aa3b67d6e9038dffe713a6"
    end

    on_arm do
      url "https://github.com/entur/openapi-validator-cli/releases/download/v#{version}/oav-#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "9a2550c9d5f29afcaf963cf78dc493dfdb88e3c19278c4d24980de0cf67a0aaf"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/entur/openapi-validator-cli/releases/download/v#{version}/oav-#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "27f8494e6a508648ce1d16fd2f5091f20691893322e0c52084362ae2e4928608"
    end
  end

  def install
    bin.install "oav"
    generate_completions_from_executable(bin/"oav", "completions", "generate")
  end

  test do
    assert_match "OpenAPI Validator", shell_output("#{bin}/oav --help")
  end
end
