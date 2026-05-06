use crate::arc_string::ArcString;
use crate::rivetx_string::RivetxString;
use crate::spawnx::tokio_spawn;
use std::sync::Arc;

fn into_rivetx_string(s: impl Into<RivetxString>) -> RivetxString {
    s.into()
}

fn from_rivetx_string(s: RivetxString) -> RivetxString {
    s
}

fn from_ref_rivetx_string(s: &RivetxString) -> RivetxString {
    s.clone()
}

pub fn rivetx_string_tests() -> anyhow::Result<()> {
    let owned_str = RivetxString::from_str("&str");
    let owned_string = RivetxString::from("String".to_string());
    let shared_str = RivetxString::from(Arc::<str>::from("Arc<str>"));
    let shared_string = RivetxString::from(Arc::new("Arc<String>".to_string()));
    let arc_string = RivetxString::from(ArcString::default());
    let static_str = RivetxString::from("&str");
    let into_rivetx_string = into_rivetx_string("&str");
    let from_rivetx_string = from_rivetx_string("&str".into());
    let from_ref_rivetx_string = from_ref_rivetx_string(&"&str".into());
    log::info!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}",
        owned_str.as_str(),
        owned_string.as_str(),
        shared_str.as_str(),
        shared_string.as_str(),
        arc_string.as_str(),
        static_str.as_str(),
        into_rivetx_string.as_str(),
        from_rivetx_string.as_str(),
        from_ref_rivetx_string.as_str(),
    );

    tokio_spawn(async move {
        log::info!(
            "{}|{}|{}|{}|{}|{}|{}{}|{}",
            owned_str.as_str(),
            owned_string.as_str(),
            shared_str.as_str(),
            shared_string.as_str(),
            arc_string.as_str(),
            static_str.as_str(),
            into_rivetx_string.as_str(),
            from_rivetx_string.as_str(),
            from_ref_rivetx_string.as_str(),
        );
        let owned_str = owned_str.into_arc_string();
        let owned_string = owned_string.into_arc_string();
        let shared_str = shared_str.into_arc_string();
        let shared_string = shared_string.into_arc_string();
        let arc_string = arc_string.into_arc_string();
        let static_str = static_str.into_arc_string();
        let into_rivetx_string = into_rivetx_string.into_arc_string();
        let from_rivetx_string = from_rivetx_string.into_arc_string();
        let from_ref_rivetx_string = from_ref_rivetx_string.into_arc_string();
        log::info!(
            "{}|{}|{}|{}|{}|{}|{}{}|{}",
            owned_str.as_str(),
            owned_string.as_str(),
            shared_str.as_str(),
            shared_string.as_str(),
            arc_string.as_str(),
            static_str.as_str(),
            into_rivetx_string.as_str(),
            from_rivetx_string.as_str(),
            from_ref_rivetx_string.as_str(),
        );
        Ok(())
    });

    Ok(())
}
