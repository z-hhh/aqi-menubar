use crate::level::Level;
use tauri::image::Image;

/// US EPA AQI category colors, plus grey for "no reading". The bands live in
/// `level`, so the dot and the level word always describe the same reading.
pub fn color_for(aqi: Option<i32>) -> [u8; 3] {
    match Level::of(aqi) {
        Level::Good => [0x00, 0xc8, 0x53],
        Level::Moderate => [0xff, 0xd6, 0x0a],
        Level::Sensitive => [0xff, 0x7e, 0x00],
        Level::Unhealthy => [0xff, 0x3b, 0x30],
        Level::VeryUnhealthy => [0x8f, 0x3f, 0x97],
        Level::Hazardous => [0x7e, 0x00, 0x23],
        Level::Unknown => [0x9a, 0x9a, 0x9a],
    }
}

/// Menu bar text colours, one palette per appearance. The EPA fills are made
/// for large areas, not for 13 pt type: yellow on white is unreadable and dark
/// green on a dark menu bar disappears, so each band gets a text-safe tone.
/// `None` means "leave the system label colour alone" (no reading yet).
pub fn text_color(aqi: Option<i32>, dark: bool) -> Option<[u8; 3]> {
    let level = Level::of(aqi);
    Some(match (level, dark) {
        (Level::Unknown, _) => return None,
        (Level::Good, false) => [0x10, 0x74, 0x2f],
        (Level::Good, true) => [0x4c, 0xd9, 0x6c],
        (Level::Moderate, false) => [0x8a, 0x5f, 0x00],
        (Level::Moderate, true) => [0xff, 0xd6, 0x0a],
        (Level::Sensitive, false) => [0xc4, 0x50, 0x00],
        (Level::Sensitive, true) => [0xff, 0x9f, 0x2e],
        (Level::Unhealthy, false) => [0xc6, 0x1b, 0x14],
        (Level::Unhealthy, true) => [0xff, 0x69, 0x5e],
        (Level::VeryUnhealthy, false) => [0x7a, 0x2e, 0x86],
        (Level::VeryUnhealthy, true) => [0xc4, 0x86, 0xcc],
        (Level::Hazardous, false) => [0x7e, 0x00, 0x23],
        (Level::Hazardous, true) => [0xff, 0x7a, 0x95],
    })
}

/// A filled, antialiased circle used as the Windows tray icon and as the
/// macOS menu bar dot. Windows cannot render text in the notification area, so
/// there the icon carries the category colour and the number lives in the
/// tooltip and the panel.
///
/// `diameter` is the fraction of the canvas the dot fills; the rest is the
/// transparent margin AppKit's status button keeps around it.
pub fn dot_image(color: [u8; 3], size: u32, diameter: f32) -> Image<'static> {
    let size_f = size as f32;
    let center = (size_f - 1.0) / 2.0;
    let radius = size_f * (diameter / 2.0);
    let mut buf = vec![0u8; (size * size * 4) as usize];

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let dist = (dx * dx + dy * dy).sqrt();
            let alpha = ((radius - dist + 0.5).clamp(0.0, 1.0) * 255.0) as u8;
            let idx = ((y * size + x) * 4) as usize;
            buf[idx] = color[0];
            buf[idx + 1] = color[1];
            buf[idx + 2] = color[2];
            buf[idx + 3] = alpha;
        }
    }

    Image::new_owned(buf, size, size)
}

/// An empty image, used as the status item's icon when the dot is hidden.
///
/// AppKit puts a status item's title through its vibrancy compositor when the
/// item carries no image at all. On a display that does not hold the menu bar
/// focus that pushes the text a long way toward white: measured on a 1x probe
/// display, magenta `#FF00FF` came out `#FF71FF` (44% white), and the same
/// text on an item carrying any image -- even a fully transparent one -- only
/// dimmed to ~60% opacity with its hue intact (`#FFD600` -> `#BDAD3B`). This
/// placeholder keeps colourless items on the same path as the dot, so the
/// level colour is only dimmed, never washed out.
///
/// The canvas is 1x18, not 1x1, because `tray-icon` scales every icon to an
/// 18 pt height and derives the width from the aspect ratio: a 1x1 image would
/// become 18 pt wide and leave a blank slot in front of the text. 1x18 keeps
/// the whole thing 1 pt wide, which is the least the status button accepts.
#[cfg(target_os = "macos")]
pub fn transparent_icon() -> Image<'static> {
    Image::new_owned(vec![0; 1 * 18 * 4], 1, 18)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "macos")]
    #[test]
    fn transparent_icon_is_present_but_empty() {
        let img = transparent_icon();
        // 18 tall so `tray-icon`'s 18 pt rescale makes it 1 pt wide, not 18.
        assert_eq!((img.width(), img.height()), (1, 18));
        assert!(img.rgba().iter().all(|b| *b == 0));
    }

    #[test]
    fn colors_follow_the_epa_bands() {
        assert_eq!(color_for(None), [0x9a, 0x9a, 0x9a]);
        assert_eq!(color_for(Some(-1)), [0x9a, 0x9a, 0x9a]);
        assert_eq!(color_for(Some(0)), [0x00, 0xc8, 0x53]);
        assert_eq!(color_for(Some(50)), [0x00, 0xc8, 0x53]);
        assert_eq!(color_for(Some(51)), [0xff, 0xd6, 0x0a]);
        assert_eq!(color_for(Some(150)), [0xff, 0x7e, 0x00]);
        assert_eq!(color_for(Some(201)), [0x8f, 0x3f, 0x97]);
        assert_eq!(color_for(Some(400)), [0x7e, 0x00, 0x23]);
    }

    #[test]
    fn text_colours_differ_per_appearance_and_skip_a_missing_reading() {
        assert_eq!(text_color(None, false), None);
        assert_eq!(text_color(Some(-1), true), None);
        // Light and dark palettes never coincide for a real band.
        for aqi in [0, 80, 120, 180, 250, 400] {
            let light = text_color(Some(aqi), false).unwrap();
            let dark = text_color(Some(aqi), true).unwrap();
            assert_ne!(light, dark, "aqi {aqi}");
        }
    }

    #[test]
    fn a_smaller_diameter_makes_a_lighter_dot() {
        let solid = |d: f32| {
            let img = dot_image([1, 2, 3], 36, d);
            img.rgba().chunks(4).filter(|px| px[3] > 200).count()
        };
        assert!(solid(0.56) > solid(0.28), "the default dot should be visibly smaller");
    }

    #[test]
    fn dot_buffer_has_expected_shape() {
        let img = dot_image([1, 2, 3], 32, 0.56);
        assert_eq!(img.width(), 32);
        assert_eq!(img.height(), 32);
        let raw = img.rgba();
        assert_eq!(raw.len(), 32 * 32 * 4);
        // Centre pixel is opaque, corner is transparent.
        let centre = ((16 * 32) + 16) * 4;
        assert_eq!(raw[centre + 3], 255);
        assert_eq!(raw[3], 0);
    }
}
