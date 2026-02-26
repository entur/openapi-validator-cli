class Oav < Formula
  desc "OpenAPI Validator CLI for linting, generating, and compiling OpenAPI specs locally"
  homepage "https://github.com/entur/openapi-validator-cli"
  version "0.4.0"
  license "EUPL-1.2"

  # Update version, urls, and sha256 values for each release.

  on_macos do
    on_intel do
      url "https://github.com/entur/openapi-validator-cli/releases/download/v#{version}/oav-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "dd1cde116a8e50ac91127a3bc32ad29eb1fa1aab81e01ec3d8a959db541cdeb2"
    end

    on_arm do
      url "https://github.com/entur/openapi-validator-cli/releases/download/v#{version}/oav-#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "b4168e9a4a1f9b4dfed00e95dd99bd6008050c91b6c0482a8309bfb9a2e1dc48"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/entur/openapi-validator-cli/releases/download/v#{version}/oav-#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "c2f08c13cdc6ec1e05bd54e1232f12801668120b9516826abfd9f2841bddf9e3"
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
