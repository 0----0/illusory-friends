use async_trait::async_trait;
use futures::{future::try_join_all, try_join};
use macroquad::prelude::*;
use std::path::{Path, PathBuf};

mod animated_sprite;

pub use animated_sprite::AnimatedSprite;
// use ustr::UstrMap;

#[async_trait]
pub trait Asset {
    async fn load(path: &Path) -> anyhow::Result<Self>
    where
        Self: Sized;
    fn delete(&self) {}
}

pub struct AssetWrapper<T: Asset> {
    path: PathBuf,
    cached: T,
}

impl<T> AssetWrapper<T>
where
    T: Asset,
{
    async fn new<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let cached = T::load(path.as_ref()).await?;
        let path = PathBuf::from(path.as_ref());
        Ok(Self { path, cached })
    }
    pub async fn reload(&mut self) -> anyhow::Result<()> {
        self.cached.delete();
        self.cached = T::load(self.path.as_path()).await?;
        Ok(())
    }
    pub fn get(&self) -> &T {
        &self.cached
    }
}

#[async_trait]
impl Asset for Texture2D {
    async fn load(path: &Path) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let texture = load_texture(path.to_str().unwrap()).await?;
        texture.set_filter(FilterMode::Nearest);
        Ok(texture)
    }

    fn delete(&self) {
        self.delete()
    }
}

#[derive(Copy, Clone)]
pub struct AnimatedSpriteId(usize);

impl Default for AnimatedSpriteId {
    fn default() -> Self {
        Self(0)
    }
}

impl AssetId for AnimatedSpriteId {
    type Asset = AnimatedSprite;

    fn get<'a>(&self, assets: &'a Assets) -> &'a Self::Asset {
        &assets.animated_sprites[self.0].get()
    }
}

pub trait AssetId {
    type Asset;
    fn get<'a>(&self, assets: &'a Assets) -> &'a Self::Asset;
}

pub struct Assets {
    pub char_concept: AssetWrapper<Texture2D>,
    // pub char_sprite: AssetWrapper<AnimatedSprite>,
    pub char_sprite: AnimatedSpriteId,
    pub animated_sprites: Vec<AssetWrapper<AnimatedSprite>>, // pub spritesheets: UstrMap<AssetWrapper<AnimatedSprite>>,
}

impl Assets {
    pub async fn new() -> anyhow::Result<Self> {
        // let (char_concept, char_sprite) = futures::try_join!(
        //     AssetWrapper::new("assets/charconcept.png"),
        //     AssetWrapper::new("assets/maribelle.json")
        // )?;

        let char_concept = AssetWrapper::new("assets/charconcept.png").await.unwrap();

        let animated_sprites = try_join_all([AssetWrapper::new("assets/maribelle.json")])
            .await
            .unwrap();
        Ok(Assets {
            char_concept,
            char_sprite: AnimatedSpriteId(0),
            animated_sprites, // spritesheets: Default::default(),
        })
    }

    pub fn get<T: AssetId>(&self, id: &T) -> &T::Asset {
        id.get(self)
    }

    pub async fn reload(&mut self) -> anyhow::Result<()> {
        try_join!(
            self.char_concept.reload(),
            // self.char_sprite.reload(),
            // try_join_all(self.spritesheets.values_mut().map(|v| { v.reload() }))
            try_join_all(self.animated_sprites.iter_mut().map(|s| s.reload()))
        )?;
        Ok(())
    }
}
