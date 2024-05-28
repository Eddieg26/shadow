use shadow_asset::{
    asset::{AssetDependency, AssetId, AssetType},
    tracker::AssetTrackers,
};

fn main() {
    let trackers = AssetTrackers::new();

    let id_1 = AssetId::new();
    let id_2 = AssetId::new();
    let id_3 = AssetId::new();

    let dependencies = vec![
        AssetDependency::new(id_2, AssetType::of::<()>()),
        AssetDependency::new(id_3, AssetType::of::<()>()),
    ];

    trackers.add::<()>(id_1);
    trackers.add::<()>(id_2);
    trackers.add::<()>(id_3);

    let result = trackers.load(id_1, &dependencies);
    println!("{:?}", result);
    let result = trackers.load(id_2, &vec![]);
    println!("{:?}", result);
    let result = trackers.load(id_3, &vec![]);
    println!("{:?}", result);
    println!("1: {:?}", id_1);
    println!("2: {:?}", id_2);
    println!("3: {:?}", id_3);
}
