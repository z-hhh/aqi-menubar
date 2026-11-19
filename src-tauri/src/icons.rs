use tauri::image::Image;

/// US EPA AQI category colors, plus grey for "no reading".
pub fn color_for(aqi: Option<i32>) -> [u8; 3] {
    let aqi = match aqi {
        Some(v) if v >= 0 => v,
        _ => return [0x9a, 0x9a, 0x9a],
    };
    if aqi <= 50 {
        [0x00, 0xc8, 0x53]
    } else if aqi <= 100 {
        [0xff, 0xd6, 0x0a]
    } else if aqi <= 150 {
        [0xff, 0x7e, 0x00]
    } else if aqi <= 200 {
        [0xff, 0x3b, 0x30]
    } else if aqi <= 300 {
        [0x8f, 0x3f, 0x97]
    } else {
        [0x7e, 0x00, 0x23]
    }
}

/// A filled, antialiased circle used as the Windows tray icon. Windows cannot
/// render text in the notification area, so the icon carries the category colour
/// and the number lives in the tooltip and the panel.
pub fn dot_image(color: [u8; 3], size: u32) -> Image<'static> {
    let size_f = size as f32;
    let center = (size_f - 1.0) / 2.0;
    let radius = size_f * 0.46;
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn dot_buffer_has_expected_shape() {
        let img = dot_image([1, 2, 3], 32);
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
