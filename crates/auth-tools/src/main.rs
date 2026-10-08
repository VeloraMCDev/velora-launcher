use anyhow::{bail, Result};
use std::path::PathBuf;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    let report = match arguments.as_slice() {
        [command, target] if command == "init" => velora_auth_tools::init_new(&PathBuf::from(target)).await?,
        [command, source, target] if command == "checkpoint" => {
            velora_auth_tools::checkpoint_new(&PathBuf::from(source), &PathBuf::from(target)).await?
        }
        [command, source, target] if command == "import" => {
            velora_auth_tools::import_new(&PathBuf::from(source), &PathBuf::from(target)).await?
        }
        [command, source, database, target] if command == "assets" => {
            velora_auth_tools::assets::copy_new(&PathBuf::from(source), &PathBuf::from(database), &PathBuf::from(target), None).await?
        }
        [command, source, database, target, jwt_file] if command == "assets" => {
            velora_auth_tools::assets::copy_new(&PathBuf::from(source), &PathBuf::from(database), &PathBuf::from(target), Some(&PathBuf::from(jwt_file))).await?
        }
        _ => bail!("usage: velora-auth-tools init <new-db> | checkpoint <closed-db> <new-backup-db> | import <offline-checkpoint> <new-db> | assets <closed-data-dir> <owned-db> <new-dir> [configured-jwt-file]"),
    };
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
