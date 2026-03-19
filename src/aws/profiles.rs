use aws_config::profile::ProfileFileLoadError;
use aws_runtime::env_config::file::EnvConfigFiles;
use aws_types::os_shim_internal::{Env, Fs};
use tokio::sync::OnceCell;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileInfo {
    pub name: String,
    pub region: Option<String>,
}

static PROFILES: OnceCell<Vec<ProfileInfo>> = OnceCell::const_new();

pub async fn list_profiles() -> color_eyre::Result<&'static [ProfileInfo]> {
    let profiles = PROFILES
        .get_or_try_init(|| async {
            let profile_set = aws_config::profile::load(
                &Fs::real(),
                &Env::real(),
                &EnvConfigFiles::default(),
                None,
            )
            .await?;

            let mut infos: Vec<ProfileInfo> = profile_set
                .profiles()
                .map(|name| {
                    let profile = profile_set.get_profile(name);
                    ProfileInfo {
                        name: name.to_string(),
                        region: profile.and_then(|p| p.get("region")).map(|s| s.to_string()),
                    }
                })
                .collect();

            infos.sort_by(|a, b| a.name.cmp(&b.name));
            Ok::<Vec<ProfileInfo>, ProfileFileLoadError>(infos)
        })
        .await?;

    Ok(profiles.as_slice())
}
