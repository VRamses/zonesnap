use crate::config::Zone;
use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Stroke, Transform};
use windows_sys::Win32::Foundation::{HWND, POINT, RECT, SIZE};
use windows_sys::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject,
    AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS,
    RGBQUAD,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, RegisterClassW, ShowWindow, UpdateLayeredWindow, ULW_ALPHA,
    SW_HIDE, SW_SHOWNOACTIVATE, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

const CLASS_NAME: &str = "ZoneSnapOverlay";

#[derive(Clone, Copy)]
pub struct MonGeom {
    pub rect: RECT,
    pub work: RECT,
}

impl MonGeom {
    pub fn width(&self) -> i32 {
        self.rect.right - self.rect.left
    }
    pub fn height(&self) -> i32 {
        self.rect.bottom - self.rect.top
    }
}

pub struct Overlay {
    hwnd: isize,
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

impl Overlay {
    pub fn create(hinstance: *mut core::ffi::c_void) -> Option<Overlay> {
        unsafe {
            let class = wide(CLASS_NAME);
            let wc = WNDCLASSW {
                style: 0,
                lpfnWndProc: Some(DefWindowProcW),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: hinstance,
                hIcon: std::ptr::null_mut(),
                hCursor: std::ptr::null_mut(),
                hbrBackground: std::ptr::null_mut(),
                lpszMenuName: std::ptr::null(),
                lpszClassName: class.as_ptr(),
            };
            RegisterClassW(&wc);
            let ex = WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE
                | WS_EX_TOPMOST;
            let hwnd = CreateWindowExW(
                ex,
                class.as_ptr(),
                wide("ZoneSnap").as_ptr(),
                WS_POPUP,
                0,
                0,
                0,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                hinstance,
                std::ptr::null_mut(),
            );
            if hwnd.is_null() {
                return None;
            }
            Some(Overlay {
                hwnd: hwnd as isize,
            })
        }
    }

    pub fn hwnd(&self) -> isize {
        self.hwnd
    }

    pub fn hide(&self) {
        unsafe {
            ShowWindow(self.hwnd as HWND, SW_HIDE);
        }
    }

    pub fn show(&self, mon: &MonGeom, zones: &[Zone], hover: Option<usize>, gap: i32) -> bool {
        let w = mon.width().max(1);
        let h = mon.height().max(1);
        let mut pix = match Pixmap::new(w as u32, h as u32) {
            Some(p) => p,
            None => return false,
        };

        for (i, z) in zones.iter().enumerate() {
            let r = zone_rect(mon, z, gap);
            let x0 = (r.left - mon.rect.left) as f32;
            let y0 = (r.top - mon.rect.top) as f32;
            let x1 = (r.right - mon.rect.left) as f32;
            let y1 = (r.bottom - mon.rect.top) as f32;
            if x1 - x0 < 4.0 || y1 - y0 < 4.0 {
                continue;
            }
            let r = ((x1 - x0).min(y1 - y0) / 2.0).min(14.0).max(2.0);
            let path = match rounded_rect(x0, y0, x1, y1, r) {
                Some(p) => p,
                None => continue,
            };
            let active = hover == Some(i);
            let (fill, border) = if active {
                (
                    Color::from_rgba8(52, 116, 246, 140),
                    Color::from_rgba8(232, 240, 255, 245),
                )
            } else {
                (
                    Color::from_rgba8(14, 16, 22, 150),
                    Color::from_rgba8(140, 180, 250, 190),
                )
            };
            let mut paint = Paint::default();
            paint.anti_alias = true;
            paint.set_color(fill);
            pix.fill_path(
                &path,
                &paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );
            let mut stroke_paint = Paint::default();
            stroke_paint.anti_alias = true;
            stroke_paint.set_color(border);
            let stroke = Stroke {
                width: if active { 3.0 } else { 2.0 },
                ..Default::default()
            };
            pix.stroke_path(
                &path,
                &stroke_paint,
                &stroke,
                Transform::identity(),
                None,
            );
        }

        unsafe {
            let screen = GetDC(std::ptr::null_mut());
            if screen.is_null() {
                return false;
            }
            let mem = CreateCompatibleDC(screen);
            if mem.is_null() {
                ReleaseDC(std::ptr::null_mut(), screen);
                return false;
            }
            let mut bi: BITMAPINFO = std::mem::zeroed();
            bi.bmiHeader = BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            };
            bi.bmiColors = [RGBQUAD {
                rgbBlue: 0,
                rgbGreen: 0,
                rgbRed: 0,
                rgbReserved: 0,
            }];
            let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
            let dib = CreateDIBSection(
                mem,
                &bi,
                DIB_RGB_COLORS,
                &mut bits,
                std::ptr::null_mut(),
                0,
            );
            if dib.is_null() || bits.is_null() {
                DeleteDC(mem);
                ReleaseDC(std::ptr::null_mut(), screen);
                return false;
            }
            let old = SelectObject(mem, dib);

            let src = pix.data();
            let dst =
                std::slice::from_raw_parts_mut(bits as *mut u8, (w as usize) * (h as usize) * 4);
            for i in (0..src.len()).step_by(4) {
                dst[i] = src[i + 2];
                dst[i + 1] = src[i + 1];
                dst[i + 2] = src[i];
                dst[i + 3] = src[i + 3];
            }

            let dst_pt = POINT {
                x: mon.rect.left,
                y: mon.rect.top,
            };
            let size = SIZE { cx: w, cy: h };
            let src_pt = POINT { x: 0, y: 0 };
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            let ok = UpdateLayeredWindow(
                self.hwnd as HWND,
                screen,
                &dst_pt,
                &size,
                mem,
                &src_pt,
                0,
                &blend,
                ULW_ALPHA,
            );
            SelectObject(mem, old);
            DeleteObject(dib);
            DeleteDC(mem);
            ReleaseDC(std::ptr::null_mut(), screen);
            if ok == 0 {
                crate::log("overlay: UpdateLayeredWindow falhou");
                return false;
            }
            ShowWindow(self.hwnd as HWND, SW_SHOWNOACTIVATE);
            true
        }
    }
}

fn rounded_rect(x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> Option<tiny_skia::Path> {
    let mut pb = PathBuilder::new();
    pb.move_to(x0 + r, y0);
    pb.line_to(x1 - r, y0);
    pb.quad_to(x1, y0, x1, y0 + r);
    pb.line_to(x1, y1 - r);
    pb.quad_to(x1, y1, x1 - r, y1);
    pb.line_to(x0 + r, y1);
    pb.quad_to(x0, y1, x0, y1 - r);
    pb.line_to(x0, y0 + r);
    pb.quad_to(x0, y0, x0 + r, y0);
    pb.close();
    pb.finish()
}

pub fn zone_rect(mon: &MonGeom, z: &Zone, gap: i32) -> RECT {
    let w = (mon.work.right - mon.work.left) as f32;
    let h = (mon.work.bottom - mon.work.top) as f32;
    let half = gap / 2;
    let rest = gap - half;
    RECT {
        left: mon.work.left + (z.x * w).round() as i32 + half,
        top: mon.work.top + (z.y * h).round() as i32 + half,
        right: mon.work.left + ((z.x + z.w) * w).round() as i32 - rest,
        bottom: mon.work.top + ((z.y + z.h) * h).round() as i32 - rest,
    }
}

pub fn hit_zone(mon: &MonGeom, zones: &[Zone], gap: i32, pt: POINT) -> Option<usize> {
    zones.iter().position(|z| {
        let r = zone_rect(mon, z, gap);
        pt.x >= r.left && pt.x < r.right && pt.y >= r.top && pt.y < r.bottom
    })
}
