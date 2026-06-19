use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

pub const BUNDLED_TEXT_FONT_REF: &str = "font://bundled/noto-sans-cjk-sc-regular";
pub const BUNDLED_TEXT_FONT_FAMILY: &str = "Noto Sans CJK SC";
pub const BUNDLED_TEXT_FONT_STYLE: &str = "Regular";
pub const BUNDLED_TEXT_FONT_WEIGHT: u16 = 400;
pub const BUNDLED_TEXT_FONT_RELATIVE_PATH: &str =
    "assets/fonts/noto-sans-cjk-sc/NotoSansCJKsc-Regular.otf";
pub const BUNDLED_TEXT_FONT_LICENSE_PATH: &str = "assets/fonts/noto-sans-cjk-sc/OFL.txt";
pub const BUNDLED_TEXT_FONT_LICENSE_SPDX: &str = "OFL-1.1";
pub const BUNDLED_TEXT_FONT_COVERAGE_SAMPLE: &str = "标题字幕测试第一句";

const BUNDLED_FONTS: &[BundledFontRegistryEntry] = &[BundledFontRegistryEntry {
    font_ref: BUNDLED_TEXT_FONT_REF,
    family: BUNDLED_TEXT_FONT_FAMILY,
    style: BUNDLED_TEXT_FONT_STYLE,
    weight: BUNDLED_TEXT_FONT_WEIGHT,
    relative_path: BUNDLED_TEXT_FONT_RELATIVE_PATH,
    license_spdx: BUNDLED_TEXT_FONT_LICENSE_SPDX,
    license_path: BUNDLED_TEXT_FONT_LICENSE_PATH,
    coverage_sample: BUNDLED_TEXT_FONT_COVERAGE_SAMPLE,
}];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BundledFontRegistryEntry {
    pub font_ref: &'static str,
    pub family: &'static str,
    pub style: &'static str,
    pub weight: u16,
    pub relative_path: &'static str,
    pub license_spdx: &'static str,
    pub license_path: &'static str,
    pub coverage_sample: &'static str,
}

impl BundledFontRegistryEntry {
    pub fn font_path(&self, repository_root: &Path) -> PathBuf {
        repository_root.join(self.relative_path)
    }

    pub fn license_file_path(&self, repository_root: &Path) -> PathBuf {
        repository_root.join(self.license_path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundledFontValidation {
    pub font_ref: &'static str,
    pub covered_sample: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontRegistryError {
    message: String,
}

impl FontRegistryError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for FontRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for FontRegistryError {}

pub fn bundled_font_registry() -> &'static [BundledFontRegistryEntry] {
    BUNDLED_FONTS
}

pub fn bundled_text_font() -> &'static BundledFontRegistryEntry {
    &BUNDLED_FONTS[0]
}

pub fn resolve_bundled_font(font_ref: &str) -> Option<&'static BundledFontRegistryEntry> {
    BUNDLED_FONTS
        .iter()
        .find(|entry| entry.font_ref == font_ref)
}

pub fn repository_root_from_manifest() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn bundled_text_font_path() -> PathBuf {
    bundled_text_font().font_path(&repository_root_from_manifest())
}

pub fn validate_bundled_font_registry(
    repository_root: &Path,
) -> Result<Vec<BundledFontValidation>, FontRegistryError> {
    BUNDLED_FONTS
        .iter()
        .map(|entry| validate_bundled_font_entry(repository_root, entry))
        .collect()
}

fn validate_bundled_font_entry(
    repository_root: &Path,
    entry: &'static BundledFontRegistryEntry,
) -> Result<BundledFontValidation, FontRegistryError> {
    if entry.font_ref.trim().is_empty() || !entry.font_ref.starts_with("font://bundled/") {
        return Err(FontRegistryError::new(format!(
            "invalid bundled fontRef {}",
            entry.font_ref
        )));
    }
    if entry.family.trim().is_empty() {
        return Err(FontRegistryError::new(format!(
            "bundled font {} must declare family",
            entry.font_ref
        )));
    }
    if entry.license_spdx != BUNDLED_TEXT_FONT_LICENSE_SPDX {
        return Err(FontRegistryError::new(format!(
            "bundled font {} must use OFL-1.1 license metadata",
            entry.font_ref
        )));
    }

    let font_path = entry.font_path(repository_root);
    if !font_path.is_file() {
        return Err(FontRegistryError::new(format!(
            "bundled font file missing: {}",
            font_path.display()
        )));
    }

    let license_path = entry.license_file_path(repository_root);
    let license = fs::read_to_string(&license_path).map_err(|error| {
        FontRegistryError::new(format!(
            "bundled font license missing: {}: {error}",
            license_path.display()
        ))
    })?;
    if !license.contains("SIL OPEN FONT LICENSE") {
        return Err(FontRegistryError::new(format!(
            "bundled font license file is not SIL OFL: {}",
            license_path.display()
        )));
    }

    let font_bytes = fs::read(&font_path).map_err(|error| {
        FontRegistryError::new(format!(
            "bundled font file unreadable: {}: {error}",
            font_path.display()
        ))
    })?;
    let cmap = SfntCmap::parse(&font_bytes).map_err(|error| {
        FontRegistryError::new(format!(
            "bundled font cmap unreadable: {}: {error}",
            font_path.display()
        ))
    })?;
    for character in entry.coverage_sample.chars() {
        if !cmap.contains(character as u32) {
            return Err(FontRegistryError::new(format!(
                "bundled font {} lacks glyph coverage for {character}",
                entry.font_ref
            )));
        }
    }

    Ok(BundledFontValidation {
        font_ref: entry.font_ref,
        covered_sample: entry.coverage_sample,
    })
}

struct SfntCmap<'a> {
    bytes: &'a [u8],
    subtables: Vec<CmapSubtable>,
}

#[derive(Debug, Clone, Copy)]
struct CmapSubtable {
    offset: usize,
    format: u16,
}

impl<'a> SfntCmap<'a> {
    fn parse(bytes: &'a [u8]) -> Result<Self, &'static str> {
        if bytes.len() < 12 {
            return Err("sfnt header too short");
        }
        let table_count = read_u16(bytes, 4).ok_or("missing table count")? as usize;
        let mut cmap_offset = None;
        let mut cmap_length = None;
        for index in 0..table_count {
            let record = 12 + index * 16;
            let tag = bytes
                .get(record..record + 4)
                .ok_or("table record truncated")?;
            if tag == b"cmap" {
                cmap_offset =
                    Some(read_u32(bytes, record + 8).ok_or("missing cmap offset")? as usize);
                cmap_length =
                    Some(read_u32(bytes, record + 12).ok_or("missing cmap length")? as usize);
                break;
            }
        }
        let cmap_offset = cmap_offset.ok_or("cmap table missing")?;
        let cmap_length = cmap_length.ok_or("cmap table length missing")?;
        let cmap_end = cmap_offset
            .checked_add(cmap_length)
            .ok_or("cmap table range overflow")?;
        let cmap = bytes
            .get(cmap_offset..cmap_end)
            .ok_or("cmap table outside file")?;
        let subtable_count = read_u16(cmap, 2).ok_or("missing cmap subtable count")? as usize;
        let mut subtables = Vec::new();
        for index in 0..subtable_count {
            let record = 4 + index * 8;
            let relative_offset =
                read_u32(cmap, record + 4).ok_or("missing cmap subtable offset")? as usize;
            let absolute_offset = cmap_offset
                .checked_add(relative_offset)
                .ok_or("cmap subtable range overflow")?;
            let format = read_u16(bytes, absolute_offset).ok_or("missing cmap subtable format")?;
            if matches!(format, 4 | 12) {
                subtables.push(CmapSubtable {
                    offset: absolute_offset,
                    format,
                });
            }
        }
        if subtables.is_empty() {
            return Err("no supported cmap subtable");
        }
        Ok(Self { bytes, subtables })
    }

    fn contains(&self, codepoint: u32) -> bool {
        self.subtables.iter().any(|subtable| match subtable.format {
            12 => self.contains_format_12(subtable.offset, codepoint),
            4 => self.contains_format_4(subtable.offset, codepoint),
            _ => false,
        })
    }

    fn contains_format_12(&self, offset: usize, codepoint: u32) -> bool {
        let group_count = match read_u32(self.bytes, offset + 12) {
            Some(value) => value as usize,
            None => return false,
        };
        (0..group_count).any(|index| {
            let group = offset + 16 + index * 12;
            let start = read_u32(self.bytes, group).unwrap_or(1);
            let end = read_u32(self.bytes, group + 4).unwrap_or(0);
            let glyph = read_u32(self.bytes, group + 8).unwrap_or(0);
            codepoint >= start && codepoint <= end && glyph != 0
        })
    }

    fn contains_format_4(&self, offset: usize, codepoint: u32) -> bool {
        if codepoint > u32::from(u16::MAX) {
            return false;
        }
        let seg_count = match read_u16(self.bytes, offset + 6) {
            Some(value) => usize::from(value / 2),
            None => return false,
        };
        let code = codepoint as u16;
        let end_codes = offset + 14;
        let start_codes = end_codes + seg_count * 2 + 2;
        let id_deltas = start_codes + seg_count * 2;
        let id_range_offsets = id_deltas + seg_count * 2;
        (0..seg_count).any(|index| {
            let end = read_u16(self.bytes, end_codes + index * 2).unwrap_or(0);
            let start = read_u16(self.bytes, start_codes + index * 2).unwrap_or(u16::MAX);
            if code < start || code > end {
                return false;
            }
            let range_offset_position = id_range_offsets + index * 2;
            let range_offset = read_u16(self.bytes, range_offset_position).unwrap_or(0);
            if range_offset == 0 {
                let delta = read_i16(self.bytes, id_deltas + index * 2).unwrap_or(0);
                return ((i32::from(code) + i32::from(delta)) & 0xffff) != 0;
            }
            let glyph_offset =
                range_offset_position + usize::from(range_offset) + usize::from(code - start) * 2;
            read_u16(self.bytes, glyph_offset).unwrap_or(0) != 0
        })
    }
}

fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    let value = bytes.get(offset..offset + 2)?;
    Some(u16::from_be_bytes([value[0], value[1]]))
}

fn read_i16(bytes: &[u8], offset: usize) -> Option<i16> {
    let value = bytes.get(offset..offset + 2)?;
    Some(i16::from_be_bytes([value[0], value[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let value = bytes.get(offset..offset + 4)?;
    Some(u32::from_be_bytes([value[0], value[1], value[2], value[3]]))
}
