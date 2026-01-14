class Oav < Formula
  desc "OpenAPI Validator CLI for linting, generating, and compiling OpenAPI specs locally"
  homepage "https://github.com/entur/openapi-validator-cli"
  version "0.1.0"
  license "EUPL-1.2"

  # Update version, urls, and sha256 values for each release.

  on_macos do
    on_intel do
      url "https://github.com/entur/openapi-validator-cli/releases/download/v#{version}/oav-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "sha256:afeecb924ba8e122aa2635b02232cf5dbd08a4530e3fd44e7eafc083a2780599"
    end

    on_arm do
      url "https://github.com/entur/openapi-validator-cli/releases/download/v#{version}/oav-#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "sha256:f1d5b519d9211ff77ace6c34621cdfe324b9858553627309e413114ebed1f8cb"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/entur/openapi-validator-cli/releases/download/v#{version}/oav-#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "sha256:5dd06069b2ce3e7e0cc2e8cdddb8c95b3fe4b38e13886387dd2bac3e4fc3be76"
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
