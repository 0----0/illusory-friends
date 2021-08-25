use async_trait::async_trait;
use futures::TryFutureExt;
use futures::{future::try_join_all, try_join};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use std::iter::FromIterator;
use std::path::{Path, PathBuf};
use ustr::{ustr, Ustr, UstrMap};

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

#[derive(Copy, Clone, Serialize, Deserialize)]
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

#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum TextureId {
    TextureId(Ustr),
    AnimatedSpriteId(AnimatedSpriteId),
}

impl From<AnimatedSpriteId> for TextureId {
    fn from(v: AnimatedSpriteId) -> Self {
        Self::AnimatedSpriteId(v)
    }
}

impl From<Ustr> for TextureId {
    fn from(v: Ustr) -> Self {
        Self::TextureId(v)
    }
}

impl Default for TextureId {
    fn default() -> Self {
        Self::TextureId("concept".into())
    }
}

impl AssetId for TextureId {
    type Asset = Texture2D;

    fn get<'a>(&self, assets: &'a Assets) -> &'a Self::Asset {
        match self {
            TextureId::TextureId(path) => &assets.textures.0[path],
            TextureId::AnimatedSpriteId(id) => &assets.get(id).src,
        }
    }
}

pub trait AssetId {
    type Asset;
    fn get<'a>(&self, assets: &'a Assets) -> &'a Self::Asset;
}

struct AssetMap<T: Asset>(UstrMap<T>);

impl<T: Asset> Default for AssetMap<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T: Asset> AssetMap<T> {
    async fn from_iter<I: IntoIterator<Item = Ustr>>(iter: I) -> anyhow::Result<Self> {
        Ok(Self(UstrMap::from_iter(
            try_join_all(
                iter.into_iter()
                    .map(|path| T::load(Path::new(path.as_str())).map_ok(move |a| (path, a))),
            )
            .await?,
        )))
    }

    async fn reload(&mut self) -> anyhow::Result<()> {
        try_join_all(self.0.iter_mut().map(|(k, v)| {
            T::load(Path::new(k.as_str())).map_ok(move |new_asset| {
                v.delete();
                *v = new_asset;
            })
        }))
        .await?;
        Ok(())
    }
}

pub struct Assets {
    pub char_concept: TextureId,
    // pub char_sprite: AssetWrapper<AnimatedSprite>,
    pub char_sprite: AnimatedSpriteId,
    pub animated_sprites: Vec<AssetWrapper<AnimatedSprite>>,
    textures: AssetMap<Texture2D>,
    pub texture_names: UstrMap<Ustr>,
}

impl Assets {
    pub async fn new() -> anyhow::Result<Self> {
        // let (char_concept, char_sprite) = futures::try_join!(
        //     AssetWrapper::new("assets/charconcept.png"),
        //     AssetWrapper::new("assets/maribelle.json")
        // )?;

        let animated_sprites = try_join_all([AssetWrapper::new("assets/maribelle.json")])
            .await
            .unwrap();

        let texture_names = UstrMap::from_iter([
            (ustr("concept"), ustr("assets/charconcept.png")),
            (ustr("minewall"), ustr("assets/minewall.png")),
            (ustr("minefloor"), ustr("assets/minefloor.png")),
        ]);

        let textures = AssetMap::from_iter(texture_names.values().cloned()).await?;

        Ok(Assets {
            char_concept: TextureId::TextureId(texture_names[&ustr("concept")]),
            char_sprite: AnimatedSpriteId(0),
            animated_sprites, // spritesheets: Default::default(),
            textures,
            texture_names,
        })
    }

    pub fn get_texture<S>(&self, id: S) -> TextureId
    where
        S: Into<Ustr>,
    {
        TextureId::TextureId(self.texture_names[&id.into()])
    }

    pub fn get<T: AssetId>(&self, id: &T) -> &T::Asset {
        id.get(self)
    }

    pub async fn reload(&mut self) -> anyhow::Result<()> {
        try_join!(
            self.textures.reload(),
            // self.char_sprite.reload(),
            // try_join_all(self.spritesheets.values_mut().map(|v| { v.reload() }))
            try_join_all(self.animated_sprites.iter_mut().map(|s| s.reload()))
        )?;
        Ok(())
    }
}
