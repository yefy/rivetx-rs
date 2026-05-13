use anyhow::{Context, Ctx};

pub fn test_a() -> anyhow::Result<()> {
    return Err(anyhow::anyhow!("test_a"));
}

pub fn test_b() -> anyhow::Result<()> {
    test_a().ctx()?;
    Ok(())
}

pub fn test_c() -> anyhow::Result<()> {
    test_b().map_err(|e| anyhow::anyhow!("test_c err:{}", e))?;
    Ok(())
}

pub fn test_d() -> anyhow::Result<()> {
    test_c().ctx_msg("test_d")?;
    Ok(())
}

pub fn test_e() -> anyhow::Result<()> {
    test_d().context("test_e")?;
    Ok(())
}

pub fn test_f() -> anyhow::Result<()> {
    test_e().map_err(|e| anyhow::anyhow!("test_f err:{}", e))?;
    Ok(())
}

pub fn test_g() -> anyhow::Result<()> {
    test_f().with_ctx(|| "test_g")?;
    Ok(())
}

pub async fn anyhow_tests() -> anyhow::Result<()> {
    let ret = test_g().ctx_msg("do_main");
    if let Err(e) = ret {
        log::info!("anyhow_tests display:{}", e);
        log::info!("anyhow_tests debug:{:?}", e);
        log::info!("anyhow_tests full:{:#?}", e);
    }
    return Ok(());
}
