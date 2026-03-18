slint::include_modules!();

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("Starting controlleros-gui");

    let window = MainWindow::new()?;
    window.run()?;

    Ok(())
}
