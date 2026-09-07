slint::include_modules!();

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    
    let ui = MainWindow::new()?;

    ui.run()?;
    Ok(())
}
