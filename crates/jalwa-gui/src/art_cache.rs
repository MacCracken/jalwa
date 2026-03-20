//! Lazy album art loading with LRU texture cache.

use egui::TextureHandle;
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

const MAX_TEXTURES: usize = 200;

struct CacheEntry {
    texture: TextureHandle,
    /// Monotonically increasing access counter for LRU eviction.
    last_access: u64,
}

/// LRU texture cache for album art. Re-extracts art from files via lofty.
pub struct ArtCache {
    textures: HashMap<Uuid, CacheEntry>,
    /// Tracks items known to have no art (avoids repeated extraction).
    no_art: std::collections::HashSet<Uuid>,
    access_counter: u64,
}

impl ArtCache {
    pub fn new() -> Self {
        Self {
            textures: HashMap::new(),
            no_art: std::collections::HashSet::new(),
            access_counter: 0,
        }
    }

    /// Get or load album art texture for a media item.
    pub fn get(
        &mut self,
        ctx: &egui::Context,
        item_id: Uuid,
        file_path: &Path,
    ) -> Option<&TextureHandle> {
        self.access_counter += 1;

        if self.no_art.contains(&item_id) {
            return None;
        }

        if !self.textures.contains_key(&item_id) {
            if let Some(tex) = load_art(ctx, file_path, item_id) {
                self.evict_if_full();
                self.textures.insert(
                    item_id,
                    CacheEntry {
                        texture: tex,
                        last_access: self.access_counter,
                    },
                );
            } else {
                self.no_art.insert(item_id);
                return None;
            }
        }

        if let Some(entry) = self.textures.get_mut(&item_id) {
            entry.last_access = self.access_counter;
            Some(&entry.texture)
        } else {
            None
        }
    }

    /// Invalidate a specific item (e.g. after metadata update).
    pub fn invalidate(&mut self, item_id: Uuid) {
        self.textures.remove(&item_id);
        self.no_art.remove(&item_id);
    }

    fn evict_if_full(&mut self) {
        while self.textures.len() >= MAX_TEXTURES {
            if let Some((&oldest_id, _)) = self
                .textures
                .iter()
                .min_by_key(|(_, entry)| entry.last_access)
            {
                self.textures.remove(&oldest_id);
            } else {
                break;
            }
        }
    }
}

fn load_art(ctx: &egui::Context, file_path: &Path, item_id: Uuid) -> Option<TextureHandle> {
    use lofty::prelude::*;
    let tagged_file = lofty::read_from_path(file_path).ok()?;
    let tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag())?;
    let pictures = tag.pictures();
    if pictures.is_empty() {
        return None;
    }

    // Prefer front cover
    let pic = pictures
        .iter()
        .find(|p| p.pic_type() == lofty::picture::PictureType::CoverFront)
        .or_else(|| pictures.first())?;

    let data = pic.data();
    if data.is_empty() {
        return None;
    }

    let img = image::load_from_memory(data).ok()?;
    // Reject oversized images to prevent excessive memory allocation
    if img.width() > 2048 || img.height() > 2048 {
        tracing::warn!(
            id = %item_id,
            width = img.width(),
            height = img.height(),
            "Skipping oversized album art"
        );
        return None;
    }
    let rgba = img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let pixels = rgba.into_raw();

    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
    let name = format!("art-{item_id}");
    Some(ctx.load_texture(name, color_image, egui::TextureOptions::LINEAR))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_new_empty() {
        let cache = ArtCache::new();
        assert!(cache.textures.is_empty());
        assert!(cache.no_art.is_empty());
    }

    #[test]
    fn invalidate_unknown_id() {
        let mut cache = ArtCache::new();
        cache.invalidate(Uuid::new_v4()); // should not panic
    }
}
