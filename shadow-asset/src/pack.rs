use crate::{
    asset::{Asset, AssetDependency, Settings},
    bytes::AsBytes,
};

pub struct AssetPack<A: Asset, S: Settings> {
    asset: A,
    settings: S,
    dependencies: Vec<AssetDependency>,
}

impl<A: Asset, S: Settings> AssetPack<A, S> {
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        let asset_len = u32::from_bytes(&bytes[0..4])? as usize;
        let asset = A::from_bytes(&bytes[4..4 + asset_len])?;

        let settings_len = u32::from_bytes(&bytes[4 + asset_len..8 + asset_len])? as usize;
        let settings = S::from_bytes(&bytes[8 + asset_len..8 + asset_len + settings_len])?;

        let dependencies =
            Vec::<AssetDependency>::from_bytes(&bytes[8 + asset_len + settings_len..])?;

        Some(Self {
            asset,
            settings,
            dependencies,
        })
    }

    pub fn save(asset: &A, settings: &S, dependencies: Vec<AssetDependency>) -> Vec<u8> {
        let mut bytes = Vec::new();
        let asset_bytes = asset.as_bytes();
        let len = asset_bytes.len() as u32;
        bytes.extend_from_slice(&len.as_bytes());
        bytes.extend_from_slice(&asset_bytes);

        let settings_bytes = settings.as_bytes();
        let len = settings_bytes.len() as u32;
        bytes.extend_from_slice(&len.as_bytes());
        bytes.extend_from_slice(&settings_bytes);

        let dependencies_bytes = dependencies.as_bytes();
        bytes.extend_from_slice(&dependencies_bytes);

        bytes
    }

    pub fn take(self) -> (A, S, Vec<AssetDependency>) {
        (self.asset, self.settings, self.dependencies)
    }
}
