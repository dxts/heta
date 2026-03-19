use anyhow::Result;
use aws_config::profile::ProfileFileLoadError;
use aws_runtime::env_config::file::EnvConfigFiles;
use aws_types::os_shim_internal::{Env, Fs};
use tokio::sync::OnceCell;

static PROFILES: OnceCell<Vec<String>> = OnceCell::const_new();

pub async fn list_profiles() -> Result<Vec<String>> {
    let profiles = PROFILES
        .get_or_try_init(|| async {
            // load all profiles using default credentials chain
            let profile_set = aws_config::profile::load(
                &Fs::real(),
                &Env::real(),
                &EnvConfigFiles::default(),
                None,
            )
            .await?;

            let mut names: Vec<String> = profile_set.profiles().map(String::from).collect();
            names.sort();
            Ok::<Vec<String>, ProfileFileLoadError>(names)
        })
        .await?;

    Ok(profiles.to_vec())
}
