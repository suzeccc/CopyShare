use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use base64::{engine::general_purpose::STANDARD, Engine};
use image::{ColorType, ImageEncoder};
use serde::{Deserialize, Serialize};
use tauri::{image::Image, AppHandle};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::error::{AppError, AppResult};
use crate::models::ClipboardTextItem;

const CLIPBOARD_ACCESS_ATTEMPTS: usize = 12;
const CLIPBOARD_ACCESS_RETRY_DELAY: Duration = Duration::from_millis(25);

fn retry_clipboard_access_with_delay<T, F, S>(mut operation: F, mut sleep: S) -> AppResult<T>
where
    F: FnMut() -> AppResult<T>,
    S: FnMut(Duration),
{
    let mut last_error = None;
    for attempt in 0..CLIPBOARD_ACCESS_ATTEMPTS {
        match operation() {
            Ok(value) => return Ok(value),
            Err(error) => {
                last_error = Some(error);
                if attempt + 1 < CLIPBOARD_ACCESS_ATTEMPTS {
                    sleep(CLIPBOARD_ACCESS_RETRY_DELAY);
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| AppError::Clipboard("clipboard access failed".to_string())))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardFileEntry {
    pub path: String,
    pub name: String,
    pub size: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
}

pub fn read_clipboard_text(app: &AppHandle) -> AppResult<String> {
    app.clipboard()
        .read_text()
        .map_err(|error| AppError::Clipboard(error.to_string()))
}

pub fn write_clipboard_text(app: &AppHandle, text: &str) -> AppResult<()> {
    app.clipboard()
        .write_text(text.to_string())
        .map_err(|error| AppError::Clipboard(error.to_string()))
}

pub fn read_clipboard_files() -> AppResult<Vec<PathBuf>> {
    #[cfg(target_os = "windows")]
    {
        read_clipboard_file_paths()
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(Vec::new())
    }
}

pub fn clipboard_has_image_data() -> bool {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::System::DataExchange::{
            IsClipboardFormatAvailable, RegisterClipboardFormatW,
        };
        use windows::core::PCWSTR;

        const CF_DIB: u32 = 8;
        const CF_DIBV5: u32 = 17;

        if unsafe { IsClipboardFormatAvailable(CF_DIB) }.is_ok()
            || unsafe { IsClipboardFormatAvailable(CF_DIBV5) }.is_ok()
        {
            return true;
        }

        let png_format: Vec<u16> = "PNG\0".encode_utf16().collect();
        let cf_png = unsafe { RegisterClipboardFormatW(PCWSTR(png_format.as_ptr())) };
        cf_png != 0 && unsafe { IsClipboardFormatAvailable(cf_png) }.is_ok()
    }

    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

pub fn write_clipboard_files(app: &AppHandle, paths: &[PathBuf]) -> AppResult<()> {
    if paths.is_empty() {
        return Err(AppError::InvalidInput("鏂囦欢鍒楄〃涓虹┖".to_string()));
    }
    if let Some(missing) = paths.iter().find(|path| std::fs::metadata(path).is_err()) {
        return Err(AppError::InvalidInput(format!(
            "鏂囦欢涓嶅瓨鍦細{}",
            missing.to_string_lossy()
        )));
    }

    #[cfg(target_os = "windows")]
    {
        let _ = app;
        write_windows_file_paths_to_clipboard(paths)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let text = paths
            .iter()
            .map(|path| path.to_string_lossy())
            .collect::<Vec<_>>()
            .join("\n");
        write_clipboard_text(app, &text)
    }
}

pub fn file_paths_to_clipboard_content(paths: &[PathBuf]) -> AppResult<String> {
    let entries = paths
        .iter()
        .map(|path| {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .filter(|name| !name.trim().is_empty())
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| path.to_string_lossy().to_string());
            let size = std::fs::metadata(path).map(|meta| meta.len()).unwrap_or(0);
            ClipboardFileEntry {
                path: path.to_string_lossy().to_string(),
                name,
                size,
                thumbnail: crate::history::video_thumbnail_base64_for_path(path, 240).ok(),
            }
        })
        .collect::<Vec<_>>();
    serde_json::to_string(&entries).map_err(Into::into)
}

pub fn clipboard_content_to_file_entries(content: &str) -> AppResult<Vec<ClipboardFileEntry>> {
    serde_json::from_str(content).map_err(Into::into)
}

pub fn clipboard_content_to_file_paths(content: &str) -> AppResult<Vec<PathBuf>> {
    Ok(clipboard_content_to_file_entries(content)?
        .into_iter()
        .filter(|entry| !entry.path.trim().is_empty())
        .map(|entry| PathBuf::from(entry.path))
        .collect())
}

pub fn summarize_file_entries(entries: &[ClipboardFileEntry]) -> String {
    match entries {
        [] => "鏂囦欢鍒楄〃".to_string(),
        [entry] => format!("{} {}", entry.name, format_file_size(entry.size)),
        entries => format!(
            "{} 涓枃浠?{}",
            entries.len(),
            format_file_size(entries.iter().map(|entry| entry.size).sum())
        ),
    }
}

fn format_file_size(size: u64) -> String {
    if size < 1024 {
        return format!("{size} B");
    }

    let units = ["KB", "MB", "GB", "TB"];
    let mut value = size as f64 / 1024.0;
    let mut unit = units[0];
    for next_unit in units.iter().skip(1) {
        if value < 1024.0 {
            break;
        }
        value /= 1024.0;
        unit = next_unit;
    }
    let precision = if value >= 10.0 { 1 } else { 2 };
    format!("{value:.precision$} {unit}")
}

pub fn summarize_file_content(content: &str) -> String {
    clipboard_content_to_file_entries(content)
        .map(|entries| summarize_file_entries(&entries))
        .unwrap_or_else(|_| "鏂囦欢鍒楄〃".to_string())
}

pub fn read_clipboard_image_base64(app: &AppHandle) -> AppResult<Option<String>> {
    if let Ok(image) = app.clipboard().read_image() {
        return image_to_png_base64(&image).map(Some);
    }

    if let Ok(Some(image)) = read_clipboard_dib_base64() {
        return Ok(Some(image));
    }

    match read_clipboard_image_file_base64() {
        Ok(image) => Ok(image),
        Err(_) => Ok(None),
    }
}

pub fn write_clipboard_image_base64(app: &AppHandle, content: &str) -> AppResult<()> {
    #[cfg(target_os = "windows")]
    {
        let _ = app;
        return write_clipboard_image_base64_windows(content);
    }

    #[cfg(not(target_os = "windows"))]
    {
    let image = png_base64_to_image(content)?;
    app.clipboard()
        .write_image(&image)
        .map_err(|error| AppError::Clipboard(error.to_string()))
    }
}

pub fn image_to_png_base64(image: &Image<'_>) -> AppResult<String> {
    let mut png = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut png);
    encoder
        .write_image(
            image.rgba(),
            image.width(),
            image.height(),
            ColorType::Rgba8.into(),
        )
        .map_err(|error| AppError::Clipboard(error.to_string()))?;
    Ok(STANDARD.encode(png))
}

pub fn png_base64_to_image(content: &str) -> AppResult<Image<'static>> {
    let bytes = STANDARD
        .decode(content)
        .map_err(|error| AppError::Clipboard(error.to_string()))?;
    let image = image::load_from_memory(&bytes)
        .map_err(|error| AppError::Clipboard(error.to_string()))?
        .to_rgba8();
    let (width, height) = image.dimensions();
    Ok(Image::new_owned(image.into_raw(), width, height))
}

#[cfg(target_os = "windows")]
fn write_clipboard_image_base64_windows(content: &str) -> AppResult<()> {
    let bytes = STANDARD
        .decode(content)
        .map_err(|error| AppError::Clipboard(error.to_string()))?;
    let image = png_base64_to_image(content)?;
    write_windows_image_to_clipboard(image.rgba(), image.width(), image.height(), &bytes)
}

#[cfg(target_os = "windows")]
fn write_windows_file_paths_to_clipboard(paths: &[PathBuf]) -> AppResult<()> {
    use windows::Win32::System::DataExchange::EmptyClipboard;

    const CF_HDROP_FORMAT: u32 = 15;

    let hdrop = build_windows_hdrop_bytes_for_paths(paths);
    unsafe {
        let _guard = open_windows_clipboard_with_retry()?;
        EmptyClipboard().map_err(|error| AppError::Clipboard(error.to_string()))?;
        set_windows_clipboard_bytes(CF_HDROP_FORMAT, &hdrop)?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn write_windows_image_to_clipboard(
    rgba: &[u8],
    width: u32,
    height: u32,
    png_bytes: &[u8],
) -> AppResult<()> {
    use windows::Win32::System::DataExchange::{
        EmptyClipboard, RegisterClipboardFormatW,
    };
    use windows::core::PCWSTR;

    const CF_DIB_FORMAT: u32 = 8;
    const CF_HDROP_FORMAT: u32 = 15;

    let dib = build_windows_dib_bytes(rgba, width, height)?;
    let html = build_windows_image_html(png_bytes);
    let temp_png_path = write_temp_clipboard_png(png_bytes)?;
    let hdrop = build_windows_hdrop_bytes(&temp_png_path);

    unsafe {
        let _guard = open_windows_clipboard_with_retry()?;
        EmptyClipboard().map_err(|error| AppError::Clipboard(error.to_string()))?;

        set_windows_clipboard_bytes(CF_DIB_FORMAT, &dib)?;

        let png_format: Vec<u16> = "PNG\0".encode_utf16().collect();
        let cf_png = RegisterClipboardFormatW(PCWSTR(png_format.as_ptr()));
        if cf_png != 0 {
            set_windows_clipboard_bytes(cf_png, png_bytes)?;
        }

        let html_format: Vec<u16> = "HTML Format\0".encode_utf16().collect();
        let cf_html = RegisterClipboardFormatW(PCWSTR(html_format.as_ptr()));
        if cf_html != 0 {
            set_windows_clipboard_bytes(cf_html, &html)?;
        }

        set_windows_clipboard_bytes(CF_HDROP_FORMAT, &hdrop)?;
    }

    Ok(())
}

#[cfg(target_os = "windows")]
unsafe fn set_windows_clipboard_bytes(format: u32, bytes: &[u8]) -> AppResult<()> {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::DataExchange::SetClipboardData;
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

    let hmem = GlobalAlloc(GMEM_MOVEABLE, bytes.len())
        .map_err(|error| AppError::Clipboard(error.to_string()))?;
    let ptr = GlobalLock(hmem);
    if ptr.is_null() {
        return Err(AppError::Clipboard("GlobalLock failed".to_string()));
    }
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr as *mut u8, bytes.len());
    let _ = GlobalUnlock(hmem);
    SetClipboardData(format, Some(HANDLE(hmem.0)))
        .map_err(|error| AppError::Clipboard(error.to_string()))?;
    Ok(())
}

#[cfg(target_os = "windows")]
struct ClipboardCloseGuard;

#[cfg(target_os = "windows")]
fn open_windows_clipboard_with_retry() -> AppResult<ClipboardCloseGuard> {
    use windows::Win32::System::DataExchange::OpenClipboard;

    retry_clipboard_access_with_delay(
        || unsafe { OpenClipboard(None) }.map_err(|error| AppError::Clipboard(error.to_string())),
        std::thread::sleep,
    )?;
    Ok(ClipboardCloseGuard)
}

#[cfg(target_os = "windows")]
impl Drop for ClipboardCloseGuard {
    fn drop(&mut self) {
        let _ = unsafe { windows::Win32::System::DataExchange::CloseClipboard() };
    }
}

#[cfg(target_os = "windows")]
fn build_windows_dib_bytes(rgba: &[u8], width: u32, height: u32) -> AppResult<Vec<u8>> {
    let expected_len = width as usize * height as usize * 4;
    if rgba.len() != expected_len {
        return Err(AppError::Clipboard("Invalid RGBA image buffer".to_string()));
    }

    let mut dib = vec![0u8; 40 + expected_len];
    dib[0..4].copy_from_slice(&40u32.to_le_bytes());
    dib[4..8].copy_from_slice(&(width as i32).to_le_bytes());
    dib[8..12].copy_from_slice(&(-(height as i32)).to_le_bytes());
    dib[12..14].copy_from_slice(&1u16.to_le_bytes());
    dib[14..16].copy_from_slice(&32u16.to_le_bytes());
    dib[20..24].copy_from_slice(&(expected_len as u32).to_le_bytes());

    for i in 0..(width as usize * height as usize) {
        let src = i * 4;
        let dst = 40 + src;
        dib[dst] = rgba[src + 2];
        dib[dst + 1] = rgba[src + 1];
        dib[dst + 2] = rgba[src];
        dib[dst + 3] = rgba[src + 3];
    }

    Ok(dib)
}

#[cfg(target_os = "windows")]
fn build_windows_image_html(png_bytes: &[u8]) -> Vec<u8> {
    let b64 = STANDARD.encode(png_bytes);
    let img_tag = format!("<img src=\"data:image/png;base64,{b64}\"/>");
    let fragment = format!("<!--StartFragment-->{img_tag}<!--EndFragment-->");
    let html_body = format!("<html><body>{fragment}</body></html>");
    let placeholder_header = "Version:0.9\r\nStartHTML:00000000\r\nEndHTML:00000000\r\nStartFragment:00000000\r\nEndFragment:00000000\r\n";
    let header_len = placeholder_header.len();
    let start_html = header_len;
    let end_html = header_len + html_body.len();
    let start_fragment = header_len + html_body.find(&fragment).unwrap_or(0);
    let end_fragment =
        header_len + html_body.find("<!--EndFragment-->").unwrap_or(0) + "<!--EndFragment-->".len();
    let header = format!(
        "Version:0.9\r\nStartHTML:{start_html:08}\r\nEndHTML:{end_html:08}\r\nStartFragment:{start_fragment:08}\r\nEndFragment:{end_fragment:08}\r\n",
    );
    let mut result = header.into_bytes();
    result.extend_from_slice(html_body.as_bytes());
    result
}

#[cfg(target_os = "windows")]
fn write_temp_clipboard_png(png_bytes: &[u8]) -> AppResult<PathBuf> {
    let mut dir = std::env::temp_dir();
    dir.push("copyshare_paste");
    std::fs::create_dir_all(&dir)?;
    dir.push(format!("paste_{}.png", uuid::Uuid::new_v4()));
    std::fs::write(&dir, png_bytes)?;
    Ok(dir)
}

#[cfg(target_os = "windows")]
fn build_windows_hdrop_bytes(path: &Path) -> Vec<u8> {
    build_windows_hdrop_bytes_for_paths(&[path.to_path_buf()])
}

#[cfg(target_os = "windows")]
fn build_windows_hdrop_bytes_for_paths(paths: &[PathBuf]) -> Vec<u8> {
    let wide_paths = paths
        .iter()
        .flat_map(|path| {
            path.to_string_lossy()
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect::<Vec<_>>()
        })
        .chain(std::iter::once(0))
        .collect::<Vec<u16>>();
    let dropfiles_size = 20usize;
    let mut data = vec![0u8; dropfiles_size + wide_paths.len() * 2];
    data[0..4].copy_from_slice(&(dropfiles_size as u32).to_le_bytes());
    data[16..20].copy_from_slice(&1u32.to_le_bytes());
    for (index, unit) in wide_paths.iter().enumerate() {
        let offset = dropfiles_size + index * 2;
        data[offset..offset + 2].copy_from_slice(&unit.to_le_bytes());
    }
    data
}

pub fn image_file_to_png_base64(path: &Path) -> AppResult<String> {
    let image = image::open(path)
        .map_err(|error| AppError::Clipboard(error.to_string()))?
        .to_rgba8();
    let (width, height) = image.dimensions();
    image_to_png_base64(&Image::new_owned(image.into_raw(), width, height))
}

fn dib_to_png_base64(dib: &[u8]) -> AppResult<String> {
    let pixel_offset = dib_pixel_offset(dib)?;
    let file_size = 14usize
        .checked_add(dib.len())
        .ok_or_else(|| AppError::Clipboard("DIB image is too large".to_string()))?;
    if file_size > u32::MAX as usize || pixel_offset > u32::MAX as usize {
        return Err(AppError::Clipboard("DIB image is too large".to_string()));
    }

    let mut bmp = Vec::with_capacity(file_size);
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&(file_size as u32).to_le_bytes());
    bmp.extend_from_slice(&0u16.to_le_bytes());
    bmp.extend_from_slice(&0u16.to_le_bytes());
    bmp.extend_from_slice(&(pixel_offset as u32).to_le_bytes());
    bmp.extend_from_slice(dib);

    let image = image::load_from_memory(&bmp)
        .map_err(|error| AppError::Clipboard(error.to_string()))?
        .to_rgba8();
    let (width, height) = image.dimensions();
    image_to_png_base64(&Image::new_owned(image.into_raw(), width, height))
}

fn dib_pixel_offset(dib: &[u8]) -> AppResult<usize> {
    let header_size = read_dib_u32(dib, 0)? as usize;
    if header_size < 12 || dib.len() < header_size {
        return Err(AppError::Clipboard("Invalid DIB header".to_string()));
    }

    let (bit_count, compression, colors_used, palette_entry_size) = if header_size == 12 {
        (read_dib_u16(dib, 10)?, 0, 0, 3usize)
    } else {
        (
            read_dib_u16(dib, 14)?,
            read_dib_u32(dib, 16)?,
            read_dib_u32(dib, 32)?,
            4usize,
        )
    };

    let color_count = if bit_count <= 8 {
        if colors_used > 0 {
            colors_used as usize
        } else {
            1usize << bit_count
        }
    } else {
        0
    };
    let mask_bytes = if header_size == 40 && compression == 3 {
        12
    } else if header_size == 40 && compression == 6 {
        16
    } else {
        0
    };
    let dib_offset = header_size
        .checked_add(mask_bytes)
        .and_then(|offset| offset.checked_add(color_count.checked_mul(palette_entry_size)?))
        .ok_or_else(|| AppError::Clipboard("Invalid DIB pixel offset".to_string()))?;

    if dib_offset > dib.len() {
        return Err(AppError::Clipboard("Invalid DIB pixel offset".to_string()));
    }

    Ok(14 + dib_offset)
}

fn read_dib_u16(dib: &[u8], offset: usize) -> AppResult<u16> {
    let bytes = dib
        .get(offset..offset + 2)
        .ok_or_else(|| AppError::Clipboard("Invalid DIB header".to_string()))?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_dib_u32(dib: &[u8], offset: usize) -> AppResult<u32> {
    let bytes = dib
        .get(offset..offset + 4)
        .ok_or_else(|| AppError::Clipboard("Invalid DIB header".to_string()))?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn image_file_path_to_png_base64(path: &Path) -> AppResult<Option<String>> {
    if !is_supported_image_file(path) {
        return Ok(None);
    }

    image_file_to_png_base64(path).map(Some)
}

fn is_supported_image_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "bmp" | "gif" | "jpeg" | "jpg" | "png" | "webp"
            )
        })
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn read_clipboard_image_file_base64() -> AppResult<Option<String>> {
    for path in read_clipboard_file_paths()? {
        if let Some(image) = image_file_path_to_png_base64(&path)? {
            return Ok(Some(image));
        }
    }

    Ok(None)
}

#[cfg(target_os = "windows")]
fn read_clipboard_dib_base64() -> AppResult<Option<String>> {
    use windows::Win32::{
        Foundation::HGLOBAL,
        System::{
            DataExchange::{
                CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
            },
            Memory::{GlobalLock, GlobalSize, GlobalUnlock},
        },
    };

    const CF_DIB: u32 = 8;
    const CF_DIBV5: u32 = 17;

    let Some(format) = [CF_DIBV5, CF_DIB]
        .into_iter()
        .find(|format| unsafe { IsClipboardFormatAvailable(*format) }.is_ok())
    else {
        return Ok(None);
    };

    struct ClipboardGuard;

    impl Drop for ClipboardGuard {
        fn drop(&mut self) {
            let _ = unsafe { CloseClipboard() };
        }
    }

    unsafe { OpenClipboard(None) }.map_err(|error| AppError::Clipboard(error.to_string()))?;
    let _guard = ClipboardGuard;
    let handle = unsafe { GetClipboardData(format) }
        .map_err(|error| AppError::Clipboard(error.to_string()))?;
    let global = HGLOBAL(handle.0);
    let size = unsafe { GlobalSize(global) };
    if size == 0 {
        return Ok(None);
    }

    let ptr = unsafe { GlobalLock(global) };
    if ptr.is_null() {
        return Ok(None);
    }

    let bytes = unsafe { std::slice::from_raw_parts(ptr as *const u8, size) }.to_vec();
    let _ = unsafe { GlobalUnlock(global) };
    dib_to_png_base64(&bytes).map(Some)
}

#[cfg(not(target_os = "windows"))]
fn read_clipboard_dib_base64() -> AppResult<Option<String>> {
    Ok(None)
}

#[cfg(not(target_os = "windows"))]
fn read_clipboard_image_file_base64() -> AppResult<Option<String>> {
    Ok(None)
}

#[cfg(target_os = "windows")]
fn read_clipboard_file_paths() -> AppResult<Vec<PathBuf>> {
    use std::os::windows::ffi::OsStringExt;

    use windows::Win32::{
        System::{
            DataExchange::{
                CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
            },
            Ole::CF_HDROP,
        },
        UI::Shell::{DragQueryFileW, HDROP},
    };

    struct ClipboardGuard;

    impl Drop for ClipboardGuard {
        fn drop(&mut self) {
            let _ = unsafe { CloseClipboard() };
        }
    }

    if unsafe { IsClipboardFormatAvailable(CF_HDROP.0 as u32) }.is_err() {
        return Ok(Vec::new());
    }
    unsafe { OpenClipboard(None) }.map_err(|error| AppError::Clipboard(error.to_string()))?;
    let _guard = ClipboardGuard;
    let handle = unsafe { GetClipboardData(CF_HDROP.0 as u32) }
        .map_err(|error| AppError::Clipboard(error.to_string()))?;
    let drop_handle = HDROP(handle.0);
    let count = unsafe { DragQueryFileW(drop_handle, u32::MAX, None) };
    let mut paths = Vec::new();

    for index in 0..count {
        let len = unsafe { DragQueryFileW(drop_handle, index, None) };
        if len == 0 {
            continue;
        }
        let mut buffer = vec![0u16; len as usize + 1];
        let written = unsafe { DragQueryFileW(drop_handle, index, Some(&mut buffer)) };
        if written == 0 {
            continue;
        }
        let path = std::ffi::OsString::from_wide(&buffer[..written as usize]);
        paths.push(PathBuf::from(path));
    }

    Ok(paths)
}

#[cfg(target_os = "windows")]
pub async fn read_clipboard_history_text(limit: usize) -> AppResult<Vec<ClipboardTextItem>> {
    use windows::ApplicationModel::DataTransfer::{
        Clipboard, ClipboardHistoryItemsResultStatus, StandardDataFormats,
    };

    if !Clipboard::IsHistoryEnabled().map_err(|error| AppError::Clipboard(error.to_string()))? {
        return Ok(Vec::new());
    }

    let result = Clipboard::GetHistoryItemsAsync()
        .map_err(|error| AppError::Clipboard(error.to_string()))?
        .get()
        .map_err(|error| AppError::Clipboard(error.to_string()))?;

    if result
        .Status()
        .map_err(|error| AppError::Clipboard(error.to_string()))?
        != ClipboardHistoryItemsResultStatus::Success
    {
        return Ok(Vec::new());
    }

    let text_format = StandardDataFormats::Text()
        .map_err(|error| AppError::Clipboard(error.to_string()))?;
    let history_items = result
        .Items()
        .map_err(|error| AppError::Clipboard(error.to_string()))?;
    let mut items = Vec::new();

    for item in history_items {
        if items.len() >= limit {
            break;
        }

        let content = item
            .Content()
            .map_err(|error| AppError::Clipboard(error.to_string()))?;
        if !content
            .Contains(&text_format)
            .map_err(|error| AppError::Clipboard(error.to_string()))?
        {
            continue;
        }

        let text = content
            .GetTextAsync()
            .map_err(|error| AppError::Clipboard(error.to_string()))?
            .get()
            .map_err(|error| AppError::Clipboard(error.to_string()))?
            .to_string()
            .trim()
            .to_string();

        if text.is_empty() {
            continue;
        }

        items.push(ClipboardTextItem {
            id: item
                .Id()
                .map_err(|error| AppError::Clipboard(error.to_string()))?
                .to_string(),
            text,
            source_device: String::new(),
        });
    }

    Ok(items)
}

#[cfg(not(target_os = "windows"))]
pub async fn read_clipboard_history_text(_limit: usize) -> AppResult<Vec<ClipboardTextItem>> {
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_list_json_round_trips_paths() {
        let paths = vec![
            PathBuf::from(r"C:\Users\SuZe\Desktop\a.txt"),
            PathBuf::from(r"D:\QiLin\demo image.png"),
        ];

        let json = file_paths_to_clipboard_content(&paths).expect("file list json");
        let restored = clipboard_content_to_file_paths(&json).expect("file list paths");

        assert_eq!(restored, paths);
    }

    #[test]
    fn file_summary_appends_size_suffix() {
        let files = vec![
            ClipboardFileEntry {
                path: r"C:\a.txt".to_string(),
                name: "a.txt".to_string(),
                size: 1024,
                thumbnail: None,
            },
            ClipboardFileEntry {
                path: r"C:\b.bin".to_string(),
                name: "b.bin".to_string(),
                size: 2048,
                thumbnail: None,
            },
        ];

        assert_eq!(
            summarize_file_entries(&files[..1]),
            "a.txt 1.00 KB",
            "single file history should append bytes"
        );
        assert!(summarize_file_entries(&files).ends_with("3.00 KB"));
    }

    #[test]
    fn image_payload_round_trips_as_png_base64() {
        let image = tauri::image::Image::new_owned(
            vec![
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
            ],
            2,
            2,
        );

        let encoded = image_to_png_base64(&image).expect("image should encode to PNG base64");
        let decoded = png_base64_to_image(&encoded).expect("PNG base64 should decode to image");

        assert_eq!(decoded.width(), 2);
        assert_eq!(decoded.height(), 2);
        assert_eq!(decoded.rgba(), image.rgba());
    }

    #[test]
    fn image_file_payload_encodes_as_png_base64() {
        let path = std::env::temp_dir().join("copyshare-image-file-test.png");
        let image = tauri::image::Image::new_owned(
            vec![
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
            ],
            2,
            2,
        );
        let png_base64 = image_to_png_base64(&image).expect("image should encode");
        std::fs::write(
            &path,
            STANDARD
                .decode(png_base64)
                .expect("test PNG base64 should decode"),
        )
        .expect("test image should be written");

        let encoded =
            image_file_to_png_base64(&path).expect("image file should be encoded as PNG base64");
        let decoded = png_base64_to_image(&encoded).expect("encoded image should decode");

        let _ = std::fs::remove_file(path);
        assert_eq!(decoded.width(), 2);
        assert_eq!(decoded.height(), 2);
        assert_eq!(decoded.rgba(), image.rgba());
    }

    #[test]
    fn non_image_file_path_is_ignored() {
        let path = std::env::temp_dir().join("copyshare-not-image.txt");

        assert!(
            image_file_path_to_png_base64(&path)
                .expect("unsupported extension should not be an error")
                .is_none()
        );
    }

    #[test]
    fn clipboard_access_retry_survives_transient_busy_errors() {
        let mut attempts = 0;
        let result = retry_clipboard_access_with_delay(
            || {
                attempts += 1;
                if attempts < 3 {
                    Err(AppError::Clipboard("clipboard busy".to_string()))
                } else {
                    Ok("ok")
                }
            },
            |_| {},
        )
        .expect("transient clipboard errors should be retried");

        assert_eq!(result, "ok");
        assert_eq!(attempts, 3);
    }

    #[test]
    fn clipboard_access_retry_stops_after_attempt_limit() {
        let mut attempts = 0;
        let error = retry_clipboard_access_with_delay(
            || {
                attempts += 1;
                Err::<(), _>(AppError::Clipboard("clipboard busy".to_string()))
            },
            |_| {},
        )
        .expect_err("persistent clipboard errors should still fail");

        assert!(error.to_string().contains("clipboard busy"));
        assert_eq!(attempts, CLIPBOARD_ACCESS_ATTEMPTS);
    }

    #[test]
    fn windows_dib_payload_encodes_as_png_base64() {
        let mut dib = Vec::new();
        dib.extend_from_slice(&40u32.to_le_bytes());
        dib.extend_from_slice(&2i32.to_le_bytes());
        dib.extend_from_slice(&(-2i32).to_le_bytes());
        dib.extend_from_slice(&1u16.to_le_bytes());
        dib.extend_from_slice(&32u16.to_le_bytes());
        dib.extend_from_slice(&0u32.to_le_bytes());
        dib.extend_from_slice(&16u32.to_le_bytes());
        dib.extend_from_slice(&0i32.to_le_bytes());
        dib.extend_from_slice(&0i32.to_le_bytes());
        dib.extend_from_slice(&0u32.to_le_bytes());
        dib.extend_from_slice(&0u32.to_le_bytes());
        dib.extend_from_slice(&[0, 0, 255, 255]);
        dib.extend_from_slice(&[0, 255, 0, 255]);
        dib.extend_from_slice(&[255, 0, 0, 255]);
        dib.extend_from_slice(&[255, 255, 255, 255]);

        let encoded =
            dib_to_png_base64(&dib).expect("Windows DIB payload should encode to PNG base64");
        let decoded = png_base64_to_image(&encoded).expect("encoded DIB image should decode");

        assert_eq!(decoded.width(), 2);
        assert_eq!(decoded.height(), 2);
        assert_eq!(
            decoded.rgba(),
            &[
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
            ]
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_clipboard_dib_builder_converts_rgba_to_bgra() {
        let dib = build_windows_dib_bytes(&[10, 20, 30, 255], 1, 1)
            .expect("DIB bytes should build");

        assert_eq!(&dib[0..4], &40u32.to_le_bytes());
        assert_eq!(&dib[4..8], &1i32.to_le_bytes());
        assert_eq!(&dib[8..12], &(-1i32).to_le_bytes());
        assert_eq!(&dib[40..44], &[30, 20, 10, 255]);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_clipboard_html_builder_has_valid_offsets() {
        let html = String::from_utf8(build_windows_image_html(b"png")).expect("HTML is utf8");

        assert!(html.contains("Version:0.9"));
        assert!(html.contains("StartFragment:"));
        assert!(html.contains("<img src=\"data:image/png;base64,"));
        assert!(html.contains("<!--StartFragment-->"));
        assert!(html.contains("<!--EndFragment-->"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_hdrop_builder_writes_wide_path() {
        let path = PathBuf::from(r"C:\Temp\copyshare.png");
        let hdrop = build_windows_hdrop_bytes(&path);

        assert_eq!(&hdrop[0..4], &20u32.to_le_bytes());
        assert_eq!(&hdrop[16..20], &1u32.to_le_bytes());
        assert!(hdrop.ends_with(&[0, 0, 0, 0]));
    }
}
