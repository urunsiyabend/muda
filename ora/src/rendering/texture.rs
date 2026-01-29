use std::sync::atomic::{AtomicU64, Ordering};
use std::path::PathBuf;
use std::collections::HashMap;
use lru::LruCache;
use std::num::NonZeroUsize;

static NEXT_TEXTURE_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextureId(u64);

impl TextureId {
    pub fn new() -> Self {
        Self(NEXT_TEXTURE_ID.fetch_add(1, Ordering::Relaxed))
    }
}

pub struct TextureEntry {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub size: (u32, u32), // width, height in pixels
}

impl TextureEntry {
    pub fn destroy(self) {
        self.texture.destroy();
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ImageSource {
    File(PathBuf),
    Bytes(u64), // Hash of bytes for cache key
}

pub struct TextureCache {
    cache: LruCache<ImageSource, TextureId>,
    textures: HashMap<TextureId, TextureEntry>,
    capacity: usize,
}

impl TextureCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: LruCache::new(NonZeroUsize::new(capacity).unwrap()),
            textures: HashMap::new(),
            capacity,
        }
    }

    /// Get texture if cached
    pub fn get(&mut self, source: &ImageSource) -> Option<TextureId> {
        self.cache.get(source).copied()
    }

    /// Get texture entry by id
    pub fn get_entry(&self, id: TextureId) -> Option<&TextureEntry> {
        self.textures.get(&id)
    }

    /// Insert a new texture, evicting LRU if at capacity
    pub fn insert(&mut self, source: ImageSource, entry: TextureEntry) -> TextureId {
        let id = TextureId::new();

        // Check if we need to evict
        if self.cache.len() >= self.capacity {
            if let Some((_, evicted_id)) = self.cache.pop_lru() {
                // CRITICAL: Destroy GPU resources
                if let Some(evicted_entry) = self.textures.remove(&evicted_id) {
                    evicted_entry.destroy();
                    log::debug!("TextureCache: Evicted texture {:?}", evicted_id);
                }
            }
        }

        self.cache.put(source, id);
        self.textures.insert(id, entry);
        id
    }

    /// Clear all textures
    pub fn clear(&mut self) {
        for (_, entry) in self.textures.drain() {
            entry.destroy();
        }
        self.cache.clear();
    }
}

impl Drop for TextureCache {
    fn drop(&mut self) {
        self.clear();
    }
}
