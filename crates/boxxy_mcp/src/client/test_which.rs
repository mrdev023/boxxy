use anyhow::Result;

pub fn test_print_npx() -> Result<()> {
    let cmd = rmcp::transport::which_command("npx")?;
    println!("DEBUG: which_command(npx) program: {:?}", cmd.get_program());
    println!("DEBUG: which_command(npx) args: {:?}", cmd.get_args().collect::<Vec<_>>());
    Ok(())
}
