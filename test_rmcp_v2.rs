use rmcp::transport::which_command;
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut tokio_cmd = which_command("npx")?;
    let std_cmd: Command = tokio_cmd.into();
    println!("Std Program: {:?}", std_cmd.get_program());
    println!("Std Args: {:?}", std_cmd.get_args().collect::<Vec<_>>());
    Ok(())
}
