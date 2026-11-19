cask "aqi-menubar" do
  version "0.1.0"
  # `:no_check` only until the first tagged release; bump.sh pins the real
  # checksum from then on.
  sha256 :no_check

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
