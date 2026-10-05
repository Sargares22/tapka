//! The icon Windows itself shows for a program, a Store app, a file or a folder, as PNG bytes.
//! The panel carries no marks of other products: an item's picture comes from the program on the
//! user's machine.

use windows::core::HSTRING;
use windows::Win32::Foundation::SIZE;
use windows::Win32::Graphics::Gdi::{
    DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS,
};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Shell::{IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_ICONONLY};

/// The icon of what `target` names, `size` px square: a path to an exe, a file or a folder, or a
/// `shell:AppsFolder\…` address. `None` for web addresses and for anything Windows cannot find.
pub fn png(target: &str, size: i32) -> Option<Vec<u8>> {
    if target.contains("://") {
        return None;
    }
    // The shell reads paths with backslashes only
    let target = target.trim().replace('/', "\\");
    let (width, height, mut pixels) = unsafe {
        // The shell needs COM on this thread; a thread that already has it answers with an error
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let factory: IShellItemImageFactory = SHCreateItemFromParsingName(&HSTRING::from(target.as_str()), None).ok()?;
        let bitmap = factory.GetImage(SIZE { cx: size, cy: size }, SIIGBF_ICONONLY).ok()?;
        let mut shape = BITMAP::default();
        GetObjectW(bitmap.into(), std::mem::size_of::<BITMAP>() as i32, Some(&mut shape as *mut _ as *mut _));
        let (width, height) = (shape.bmWidth, shape.bmHeight);
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // top row first
                biPlanes: 1,
                biBitCount: 32,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut pixels = vec![0u8; (width.max(0) * height.max(0) * 4) as usize];
        let dc = GetDC(None);
        let rows = GetDIBits(dc, bitmap, 0, height as u32, Some(pixels.as_mut_ptr() as *mut _), &mut info, DIB_RGB_COLORS);
        ReleaseDC(None, dc);
        let _ = DeleteObject(bitmap.into());
        if width <= 0 || height <= 0 || rows != height {
            return None;
        }
        (width as u32, height as u32, pixels)
    };
    // Windows hands out BGRA with the colours already multiplied by alpha; PNG wants plain RGBA.
    // An old icon without an alpha channel comes with alpha 0 everywhere and is opaque.
    let opaque = pixels.chunks_exact(4).all(|p| p[3] == 0);
    for p in pixels.chunks_exact_mut(4) {
        p.swap(0, 2);
        if opaque {
            p[3] = 255;
        } else if p[3] > 0 && p[3] < 255 {
            let alpha = p[3] as u32;
            for c in &mut p[..3] {
                *c = ((*c as u32 * 255 + alpha / 2) / alpha).min(255) as u8;
            }
        }
    }
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().ok()?.write_image_data(&pixels).ok()?;
    Some(out)
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_program_has_an_icon_and_an_address_has_none() {
        let windir = std::env::var("WINDIR").unwrap();
        let icon = super::png(&format!("{windir}\\explorer.exe"), 64).expect("explorer.exe has an icon");
        assert_eq!(&icon[..8], b"\x89PNG\r\n\x1a\n");
        // Written with forward slashes it is the same program
        assert!(super::png(&format!("{windir}/explorer.exe").replace('\\', "/"), 64).is_some());
        // A folder has one too
        assert!(super::png(&windir, 64).is_some());
        assert!(super::png("https://example.com", 64).is_none());
        assert!(super::png(r"C:\no\such\program.exe", 64).is_none());
    }
}
