#!/usr/bin/env bash
# Rewrite the tap's cask for a new release.
#
#   ./bump.sh <version> <sha256-of-the-universal-dmg> <path-to-tap-checkout>
#
# Called by .github/workflows/release.yml after the build job publishes the
# draft release. Also fine to run by hand.
set -euo pipefail

version="${1:?usage: bump.sh <version> <sha256> <tap-dir>}"
sha="${2:?usage: bump.sh <version> <sha256> <tap-dir>}"
tap="${3:?usage: bump.sh <version> <sha256> <tap-dir>}"

cask="$tap/Casks/aqi-menubar.rb"
mkdir -p "$(dirname "$cask")"

cat > "$cask" <<RUBY
cask "aqi-menubar" do
  version "${version}"
  sha256 "${sha}"

  url "https://github.com/z-hhh/aqi-menubar/releases/download/v#{version}/AQI%20Menubar_#{version}_universal.dmg",
      verified: "github.com/z-hhh/aqi-menubar/"
  name "AQI Menubar"
  desc "Air quality index in the menu bar, powered by aqicn.org"
  homepage "https://github.com/z-hhh/aqi-menubar"

  livecheck do
    url :url
    strategy :github_latest
  end

  depends_on macos: ">= :big_sur"

  app "AQI Menubar.app"

  zap trash: [
    "~/Library/Application Support/app.aqimenubar.desktop",
    "~/Library/Caches/app.aqimenubar.desktop",
    "~/Library/Preferences/app.aqimenubar.desktop.plist",
  ]
end
RUBY

echo "wrote $cask (version ${version}, sha256 ${sha})"
