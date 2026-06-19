mod macos;
mod windows;

#[cfg(target_os = "macos")]
pub use macos::macos_system_metal_device_id;
pub use macos::{
    MacosMediaReader, MacosMediaSession, MacosRegisteredTextureLease, MacosTextureInteropPolicy,
    MacosVideoDecoder, probe_macos_media_io_capabilities, select_macos_texture_interop_fallback,
};
pub use windows::{
    WindowsMediaReader, WindowsMediaSession, WindowsTextureInteropPolicy, WindowsVideoDecoder,
    probe_windows_media_io_capabilities, select_windows_texture_interop_fallback,
};
