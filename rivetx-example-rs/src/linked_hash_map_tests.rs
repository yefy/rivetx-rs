use linked_hash_map::LinkedHashMap;
use rivetx_core_rs::arc_string::ArcString;

pub async fn linked_hash_map_tests() -> anyhow::Result<()> {
    let mut m: LinkedHashMap<ArcString, i32> = LinkedHashMap::new();
    m.insert(ArcString::from("11"), 11);
    m.insert(ArcString::from("11"), 111);
    m.insert(ArcString::from("12"), 12);
    m.insert(ArcString::from("1"), 1);
    m.insert(ArcString::from("13"), 13);
    m.insert(ArcString::from("2"), 2);
    m.insert(ArcString::from("12"), 122);
    m.insert(ArcString::from("13"), 133);
    m[&ArcString::from("1")] = 11;
    //m[&ArcString::from("14")] = 14;
    m.entry(ArcString::from("14")).or_insert(14);
    m.insert(ArcString::from("15"), 15);
    m.entry(ArcString::from("14")).or_insert(144);
    let keys = m
        .keys()
        .map(|data| data.clone())
        .collect::<Vec<ArcString>>();
    println!("m:{:?}", m);
    println!("keys:{:?}", keys);

    m.pop_front();
    let keys = m
        .keys()
        .map(|data| data.clone())
        .collect::<Vec<ArcString>>();
    println!("m:{:?}", m);
    println!("keys:{:?}", keys);

    m.pop_back();
    let keys = m
        .keys()
        .map(|data| data.clone())
        .collect::<Vec<ArcString>>();
    println!("m:{:?}", m);
    println!("keys:{:?}", keys);

    Ok(())
}
