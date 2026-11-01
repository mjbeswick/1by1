use fltk::enums::Color;

/// Every colour the UI needs, resolved from the system appearance.
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub window: Color,
    pub panel: Color,
    pub text: Color,
    pub dim: Color,
    pub bar: Color,
    pub button: Color,
    pub button_hi: Color,
    pub list_bg: Color,
    pub list_bg_alt: Color,
    pub list_text: Color,
    pub list_sel: Color,
    pub list_sel_text: Color,
    pub current: Color,
    pub current_text: Color,
    pub grid: Color,
    pub header: Color,
    pub header_text: Color,
    pub slider: Color,
    pub slider_knob: Color,
    pub big_bg: Color,
    pub big_text: Color,
    pub big_dim: Color,
    pub dark: bool,
}

pub fn rgb(rgb: u32) -> Color {
    // `Color::from_u32` switches to RGBA interpretation when FLTK's
    // `enable-glwindow` feature is enabled, which drops the red channel.
    Color::from_rgb(
        ((rgb >> 16) & 0xFF) as u8,
        ((rgb >> 8) & 0xFF) as u8,
        (rgb & 0xFF) as u8,
    )
}

fn c(value: u32) -> Color {
    rgb(value)
}

impl Palette {
    pub fn os(dark: bool) -> Self {
        if dark {
            Self::dark()
        } else {
            Self::light()
        }
    }

    fn light() -> Self {
        Self {
            window: c(0xF2F2F2),
            panel: c(0xE8E8E8),
            text: c(0x111111),
            dim: c(0x747474),
            bar: c(0xEDEDED),
            button: c(0xE4E4E4),
            button_hi: c(0xCECECE),
            list_bg: Color::White,
            list_bg_alt: c(0xF7F7F7),
            list_text: c(0x000000),
            list_sel: c(0x2F6FBF),
            list_sel_text: Color::White,
            current: c(0xFFF1A8),
            current_text: c(0x000000),
            grid: c(0xD6D6D6),
            header: c(0xE6E6E6),
            header_text: c(0x282828),
            slider: c(0xC8C8C8),
            slider_knob: c(0xFAFAFA),
            big_bg: c(0x121518),
            big_text: c(0xE8F0F8),
            big_dim: c(0x92A1B0),
            dark: false,
        }
    }

    fn dark() -> Self {
        Self {
            window: c(0x1E1E1E),
            panel: c(0x262626),
            text: c(0xE8E8E8),
            dim: c(0x8C8C8C),
            bar: c(0x252525),
            button: c(0x303030),
            button_hi: c(0x474747),
            list_bg: c(0x191919),
            list_bg_alt: c(0x1E1E1E),
            list_text: c(0xE4E4E4),
            list_sel: c(0x3568A8),
            list_sel_text: c(0xFFFFFF),
            current: c(0x4A3F1E),
            current_text: c(0xFFE9A8),
            grid: c(0x333333),
            header: c(0x2A2A2A),
            header_text: c(0xCCCCCC),
            slider: c(0x3A3A3A),
            slider_knob: c(0xD0D0D0),
            big_bg: c(0x0A0A0A),
            big_text: c(0xEDEDED),
            big_dim: c(0x8A8A8A),
            dark: true,
        }
    }
}

/// True when the system is in dark appearance.
pub fn os_dark() -> bool {
    #[cfg(target_os = "macos")]
    {
        macos_dark()
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

#[cfg(target_os = "macos")]
fn macos_dark() -> bool {
    use std::ffi::{c_char, c_void};

    type CFTypeRef = *const c_void;
    type CFStringRef = *const c_void;

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        static kCFPreferencesAnyApplication: CFStringRef;
        fn CFPreferencesCopyAppValue(key: CFStringRef, application: CFStringRef) -> CFTypeRef;
        fn CFStringCreateWithCString(
            allocator: CFTypeRef,
            c_str: *const c_char,
            encoding: u32,
        ) -> CFStringRef;
        fn CFStringGetCString(
            string: CFStringRef,
            buffer: *mut c_char,
            size: isize,
            encoding: u32,
        ) -> u8;
        fn CFRelease(cf: CFTypeRef);
    }

    const UTF8: u32 = 0x0800_0100;

    unsafe {
        let key = CFStringCreateWithCString(
            std::ptr::null(),
            b"AppleInterfaceStyle\0".as_ptr() as *const c_char,
            UTF8,
        );
        if key.is_null() {
            return false;
        }
        let value = CFPreferencesCopyAppValue(key, kCFPreferencesAnyApplication);
        CFRelease(key);
        if value.is_null() {
            return false;
        }
        let mut buf = [0i8; 32];
        let ok = CFStringGetCString(value as CFStringRef, buf.as_mut_ptr(), buf.len() as isize, UTF8);
        CFRelease(value);
        if ok == 0 {
            return false;
        }
        let text = std::ffi::CStr::from_ptr(buf.as_ptr())
            .to_string_lossy()
            .to_ascii_lowercase();
        text == "dark"
    }
}
