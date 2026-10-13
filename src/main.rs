fn main() -> Result<(), Box<dyn std::error::Error>> {
    let instance = single_instance::SingleInstance::new("viper-music")?;
    if !instance.is_single() {
        return Ok(());
    }
    viper::run()?;
    Ok(())
}
