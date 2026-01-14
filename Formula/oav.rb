class Oav < Formula
  desc "OpenAPI Validator CLI for linting, generating, and compiling OpenAPI specs locally"
  homepage "https://github.com/entur/openapi-validator-cli"
  version "0.1.0"
  license "EUPL-1.2"

  # Update version, urls, and sha256 values for each release.

  on_macos do
    on_intel do
      url "https://github.com/entur/openapi-validator-cli/releases/download/v#{version}/oav-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "5c1a01351699a48dd3112e446cfb82a775bef816b47c58622e0e178bb0f3c057"
    end

    on_arm do
      url "https://github.com/entur/openapi-validator-cli/releases/download/v#{version}/oav-#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "667132cfe9c737cfda86af2cf13a6e519744c5cd728f8b788527365b2f4b2876"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/entur/openapi-validator-cli/releases/download/v#{version}/oav-#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "e1c497398d8e677e43d719da4f385d0f95a1a01f3d23644e29e5bfee568020bf"
    end
  end

  def install
    bin.install "oav"
    bin.install "openapi-validator"
  end

  test do
    assert_match "OpenAPI Validator", shell_output("#{bin}/oav --help")
  end
end
