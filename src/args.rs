use clap::Parser;

#[derive(Parser, Debug)]
#[command(version = "0.1", about, long_about = None)]
pub struct Args {
    #[arg(short = 'd', long)]
    pub device_ip: Option<String>,
    #[arg(short = 'p', long)]
    pub device_port: Option<u32>,
    #[arg(short = 'g', long, action=clap::ArgAction::SetTrue)]
    pub use_gui: bool,
}
