fn main() -> anyhow::Result<()> {
    if std::env::consts::ARCH != "aarch64" {
        anyhow::bail!("CodexSkinLite supports only macOS Apple Silicon");
    }
    Ok(())
}
