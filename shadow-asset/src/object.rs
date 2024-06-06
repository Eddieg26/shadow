use crate::{
    asset::{Asset, AssetId, Settings},
    bytes::AsBytes,
};

pub struct AssetObject<A: Asset, S: Settings> {
    asset: A,
    settings: S,
    dependencies: Vec<AssetId>,
}

impl<A: Asset, S: Settings> AssetObject<A, S> {
    pub fn new(asset: A, settings: S, dependencies: Vec<AssetId>) -> Self {
        Self {
            asset,
            settings,
            dependencies,
        }
    }

    pub fn asset(&self) -> &A {
        &self.asset
    }

    pub fn settings(&self) -> &S {
        &self.settings
    }

    pub fn dependencies(&self) -> &[AssetId] {
        &self.dependencies
    }

    pub fn take(self) -> (A, S, Vec<AssetId>) {
        (self.asset, self.settings, self.dependencies)
    }
}

impl<A: Asset, S: Settings> AsBytes for AssetObject<A, S> {
    fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];

        let asset = self.asset.as_bytes();
        bytes.extend(asset.len().as_bytes());
        bytes.extend(asset);

        let settings = self.settings.as_bytes();
        bytes.extend(settings.len().as_bytes());
        bytes.extend(settings);

        bytes.extend(self.dependencies.as_bytes());

        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let mut offset = 0usize;

        let len = usize::from_bytes(&bytes[..8])?;
        offset += 8;
        let asset = A::from_bytes(&bytes[offset..len])?;
        offset += len;

        let len = usize::from_bytes(&bytes[offset..offset + 8])?;
        offset += 8;
        let settings = S::from_bytes(&bytes[offset..offset + len])?;
        offset += len;

        let dependencies = Vec::<AssetId>::from_bytes(&bytes[offset..])?;

        Some(Self::new(asset, settings, dependencies))
    }
}
