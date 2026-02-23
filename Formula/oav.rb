class Oav < Formula
  desc "OpenAPI Validator CLI for linting, generating, and compiling OpenAPI specs locally"
  homepage "https://github.com/entur/openapi-validator-cli"
  version "0.3.0"
  license "EUPL-1.2"

  # Update version, urls, and sha256 values for each release.

  on_macos do
    on_intel do
      url "https://github.com/entur/openapi-validator-cli/releases/download/v#{version}/oav-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "33f19c0eb5956807820c4583be877c7486b41ded37c3fa52036398914de91e6d"
    end

    on_arm do
      url "https://github.com/entur/openapi-validator-cli/releases/download/v#{version}/oav-#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "6e1351b6c02022219666f60608a26765e5bc4e170b4fdec1df3112ab175ca141"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/entur/openapi-validator-cli/releases/download/v#{version}/oav-#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "d0b6cfa6588dbf4bb9326f53758ca688f486b26fbbd1a0e8f163d2cb533360ac"
    end
  end

  def install
    bin.install "oav"
  end

  test do
    assert_match "OpenAPI Validator", shell_output("#{bin}/oav --help")
  end
end
