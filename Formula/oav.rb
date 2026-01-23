class Oav < Formula
  desc "OpenAPI Validator CLI for linting, generating, and compiling OpenAPI specs locally"
  homepage "https://github.com/entur/openapi-validator-cli"
  version "0.2.0"
  license "EUPL-1.2"

  # Update version, urls, and sha256 values for each release.

  on_macos do
    on_intel do
      url "https://github.com/entur/openapi-validator-cli/releases/download/v#{version}/oav-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "dc55491c200b5571beba1734a0a521c5d3b7deefa2f2517330e337182abb65fb"
    end

    on_arm do
      url "https://github.com/entur/openapi-validator-cli/releases/download/v#{version}/oav-#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "a6143d5a21c08ad6f860d99a5dc79ba2e9a809a7f1454c976037c6c75f91ab64"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/entur/openapi-validator-cli/releases/download/v#{version}/oav-#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "e74eb0d5e41019de9c96772ad0412da48c726a00d326408ecb11d8caed5c2c83"
    end
  end

  def install
    bin.install "oav"
  end

  test do
    assert_match "OpenAPI Validator", shell_output("#{bin}/oav --help")
  end
end
