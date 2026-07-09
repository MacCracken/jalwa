//! Lazy album art loading with LRU texture cache.

use egui::TextureHandle;
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

const MAX_TEXTURES: usize = 200;
const MAX_NO_ART: usize = 1000;

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
                self.evict_no_art_if_full();
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

    /// Number of items known to have no art (for testing).
    #[cfg(test)]
    pub fn no_art_count(&self) -> usize {
        self.no_art.len()
    }

    /// Number of cached textures (for testing).
    #[cfg(test)]
    pub fn texture_count(&self) -> usize {
        self.textures.len()
    }

    /// Clear the no_art set when it grows past the threshold.
    /// Since HashSet doesn't track insertion order, we clear the whole set
    /// and let entries be re-added on cache miss.
    fn evict_no_art_if_full(&mut self) {
        if self.no_art.len() > MAX_NO_ART {
            self.no_art.clear();
        }
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

    #[test]
    fn cache_no_art_tracking() {
        let mut cache = ArtCache::new();
        let ctx = egui::Context::default();
        let fake_id = Uuid::new_v4();
        let fake_path = std::path::PathBuf::from("/nonexistent/track.mp3");

        // First call: file doesn't exist, should return None and record in no_art
        let result = cache.get(&ctx, fake_id, &fake_path);
        assert!(result.is_none());
        assert_eq!(cache.no_art_count(), 1);

        // Second call: same id, should short-circuit via no_art set
        let result2 = cache.get(&ctx, fake_id, &fake_path);
        assert!(result2.is_none());
        assert_eq!(cache.no_art_count(), 1); // still 1, not 2

        // Different id also gets tracked
        let fake_id2 = Uuid::new_v4();
        let result3 = cache.get(&ctx, fake_id2, &fake_path);
        assert!(result3.is_none());
        assert_eq!(cache.no_art_count(), 2);
    }

    #[test]
    fn invalidate_clears_no_art() {
        let mut cache = ArtCache::new();
        let ctx = egui::Context::default();
        let fake_id = Uuid::new_v4();
        let fake_path = std::path::PathBuf::from("/nonexistent/track.mp3");

        // Mark as no-art
        let _ = cache.get(&ctx, fake_id, &fake_path);
        assert_eq!(cache.no_art_count(), 1);

        // Invalidate should clear from no_art set
        cache.invalidate(fake_id);
        assert_eq!(cache.no_art_count(), 0);
        assert_eq!(cache.texture_count(), 0);
    }

    #[test]
    fn cache_max_textures_constant() {
        // Verify the constant is within a reasonable range
        let max = MAX_TEXTURES;
        assert!(max > 0, "MAX_TEXTURES must be positive");
        assert!(max <= 1000, "MAX_TEXTURES too large: {max}");
    }

    #[test]
    fn no_art_set_eviction() {
        let mut cache = ArtCache::new();
        let ctx = egui::Context::default();
        let fake_path = std::path::PathBuf::from("/nonexistent/track.mp3");

        // Fill the no_art set past the threshold
        for _ in 0..=MAX_NO_ART {
            let id = Uuid::new_v4();
            let _ = cache.get(&ctx, id, &fake_path);
        }

        // After exceeding MAX_NO_ART, the set should have been cleared
        // (and then the last insert re-added one entry, so it's not empty
        // but it's well under the threshold).
        assert!(
            cache.no_art.len() <= 1,
            "no_art set should have been cleared after exceeding {MAX_NO_ART}, but has {} entries",
            cache.no_art.len()
        );
    }

    #[test]
    fn no_art_set_below_threshold_not_cleared() {
        let mut cache = ArtCache::new();
        let ctx = egui::Context::default();
        let fake_path = std::path::PathBuf::from("/nonexistent/track.mp3");

        // Add entries below the threshold
        for _ in 0..10 {
            let id = Uuid::new_v4();
            let _ = cache.get(&ctx, id, &fake_path);
        }
        assert_eq!(
            cache.no_art_count(),
            10,
            "no_art set should retain entries below threshold"
        );
    }

    #[test]
    fn cache_access_counter_increments() {
        let mut cache = ArtCache::new();
        let ctx = egui::Context::default();
        assert_eq!(cache.access_counter, 0);

        let _ = cache.get(&ctx, Uuid::new_v4(), std::path::Path::new("/a.mp3"));
        assert_eq!(cache.access_counter, 1);

        let _ = cache.get(&ctx, Uuid::new_v4(), std::path::Path::new("/b.mp3"));
        assert_eq!(cache.access_counter, 2);
    }
}
