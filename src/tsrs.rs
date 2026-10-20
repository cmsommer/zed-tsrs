use std::fs;
use std::path::{Path, PathBuf};

use zed_extension_api::serde_json::{Value, json};
use zed_extension_api::{self as zed, Result, settings::LspSettings};

struct TsRsExtension {
    cached_binary_path: Option<String>,
    cached_version: Option<String>,
}

const PACKAGE_NAME: &str = "tsrs";
const PLATFORM_SCOPE: &str = "@tsrs";

#[derive(Debug, Default)]
struct TsRsSettings {
    package_version: Option<String>,
}

impl TsRsSettings {
    fn from_lsp_settings(settings: &Value) -> Self {
        let package_version = settings
            .get("package_version")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Self { package_version }
    }
}

fn merge_json_value_into(source: Value, target: &mut Value) {
    match (source, target) {
        (Value::Object(source), Value::Object(target)) => {
            for (key, value) in source {
                if let Some(target) = target.get_mut(&key) {
                    merge_json_value_into(value, target);
                } else {
                    target.insert(key, value);
                }
            }
        }

        (Value::Array(source), Value::Array(target)) => {
            for value in source {
                target.push(value);
            }
        }

        (source, target) => *target = source,
    }
}

impl TsRsExtension {
    fn binary_name() -> &'static str {
        let (platform, _) = zed::current_platform();
        match platform {
            zed::Os::Windows => "tsrs.exe",
            _ => "tsrs",
        }
    }

    /// Find the native binary under whichever `@tsrs/*` optional dependency npm installed.
    fn get_native_binary_path() -> Result<PathBuf> {
        let scope_dir = PathBuf::from("node_modules").join(PLATFORM_SCOPE);
        if !scope_dir.is_dir() {
            return Err(format!(
                "No platform packages found under {}. The `tsrs` package installs a matching \
                `@tsrs/<os>-<arch>[-gnu|-musl]` optional dependency; this platform may be unsupported \
                (current npm releases cover darwin-arm64 and linux gnu/musl only).",
                scope_dir.display()
            ));
        }

        let binary_name = Self::binary_name();
        let entries = fs::read_dir(&scope_dir).map_err(|e| {
            format!(
                "Failed to read {}: {}",
                scope_dir.display(),
                e
            )
        })?;

        let mut candidates = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let binary_path = entry.path().join(binary_name);
            if binary_path.is_file() {
                candidates.push(binary_path);
            }
        }

        match candidates.len() {
            0 => Err(format!(
                "Installed `@tsrs/*` packages under {} but none contain `{}`. \
                Reinstall the extension or pin a `package_version` that supports this platform.",
                scope_dir.display(),
                binary_name
            )),
            1 => Ok(candidates.remove(0)),
            _ => {
                // Prefer a package whose name matches the current OS/arch when multiple exist
                // (e.g. leftover modules after switching machines).
                if let Some(preferred) = Self::preferred_candidate(&candidates) {
                    Ok(preferred)
                } else {
                    Ok(candidates.remove(0))
                }
            }
        }
    }

    fn preferred_candidate(candidates: &[PathBuf]) -> Option<PathBuf> {
        let (platform, arch) = zed::current_platform();
        let os = match platform {
            zed::Os::Mac => "darwin",
            zed::Os::Linux => "linux",
            zed::Os::Windows => "win32",
        };
        let arch = match arch {
            zed::Architecture::Aarch64 => "arm64",
            zed::Architecture::X8664 => "x64",
            zed::Architecture::X86 => return None,
        };
        let needle = format!("{os}-{arch}");

        candidates
            .iter()
            .find(|path| {
                path.parent()
                    .and_then(Path::file_name)
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with(&needle) || name.contains(&needle))
            })
            .cloned()
    }

    fn binary_exists(&self) -> bool {
        Self::get_native_binary_path().is_ok()
    }

    fn get_installed_version(&self) -> Option<String> {
        zed::npm_package_installed_version(PACKAGE_NAME)
            .ok()
            .flatten()
    }

    fn should_install_or_update(&self, target_version: &str) -> bool {
        if !self.binary_exists() {
            return true;
        }

        match self.get_installed_version() {
            Some(installed_version) => installed_version != target_version,
            None => true,
        }
    }

    fn install_package(
        &mut self,
        id: &zed::LanguageServerId,
        custom_version: Option<&str>,
    ) -> Result<()> {
        zed::set_language_server_installation_status(
            id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        let target_version = match custom_version {
            Some(version) => version.to_string(),
            None => zed::npm_package_latest_version(PACKAGE_NAME)?,
        };

        if self.should_install_or_update(&target_version) {
            zed::set_language_server_installation_status(
                id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            let result = zed::npm_install_package(PACKAGE_NAME, &target_version);
            if let Err(error) = result
                && !self.binary_exists()
            {
                return Err(error);
            }
        }

        let binary_path = Self::get_native_binary_path()
            .map_err(|e| format!("Failed to locate native binary after installation: {}", e))?;

        self.cached_binary_path = Some(binary_path.to_string_lossy().to_string());
        self.cached_version = Some(target_version);

        Ok(())
    }

    fn binary_path(
        &mut self,
        id: &zed::LanguageServerId,
        package_version: Option<&str>,
    ) -> Result<String> {
        if let Some(cached_path) = self.cached_binary_path.as_ref()
            && fs::metadata(cached_path).is_ok_and(|stat| stat.is_file())
        {
            return Ok(cached_path.clone());
        }

        self.install_package(id, package_version)?;

        let binary_path = Self::get_native_binary_path()
            .map_err(|e| format!("Failed to locate native binary: {}", e))?;

        Ok(binary_path.to_string_lossy().to_string())
    }
}

impl zed::Extension for TsRsExtension {
    fn new() -> Self {
        Self {
            cached_binary_path: None,
            cached_version: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let lsp_settings = LspSettings::for_worktree(language_server_id.as_ref(), worktree).ok();

        let settings = lsp_settings
            .as_ref()
            .and_then(|s| s.settings.as_ref())
            .map(TsRsSettings::from_lsp_settings)
            .unwrap_or_default();

        let env = lsp_settings
            .as_ref()
            .and_then(|s| s.binary.as_ref())
            .and_then(|binary| binary.env.clone());

        let package_version = settings.package_version.as_deref();
        let executable_path = self.binary_path(language_server_id, package_version)?;

        Ok(zed::Command {
            command: std::env::current_dir()
                .map_err(|e| e.to_string())?
                .join(executable_path)
                .to_string_lossy()
                .into_owned(),
            args: vec!["--lsp".into(), "--stdio".into()],
            env: env.into_iter().flat_map(|env| env.into_iter()).collect(),
        })
    }

    fn language_server_initialization_options(
        &mut self,
        server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<Value>> {
        let mut initialization_options = json!({
            "codeLensShowLocationsCommandName": "editor.action.showReferences"
        });

        if let Some(user_initialization_options) =
            LspSettings::for_worktree(server_id.as_ref(), worktree)?
                .initialization_options
        {
            merge_json_value_into(user_initialization_options, &mut initialization_options);
        }

        Ok(Some(initialization_options))
    }

    fn language_server_workspace_configuration(
        &mut self,
        server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<Value>> {
        let config = json!({
            "inlayHints": {
                "parameterNames": {
                    "enabled": "all",
                    "suppressWhenArgumentMatchesName": false
                },
                "parameterTypes": {
                    "enabled": true
                },
                "variableTypes": {
                    "enabled": true,
                    "suppressWhenTypeMatchesName": false
                },
                "propertyDeclarationTypes": {
                    "enabled": true
                },
                "functionLikeReturnTypes": {
                    "enabled": true
                },
                "enumMemberValues": {
                    "enabled": true
                }
            },
            "implementationsCodeLens": {
                "enabled": true,
                "showOnAllClassMethods": true,
                "showOnInterfaceMethods": true
            },
            "referencesCodeLens": {
                "enabled": true,
                "showOnAllFunctions": true
            }
        });

        let mut settings = json!({
            "typescript": config.clone(),
            "javascript": config
        });

        if let Some(user_settings) =
            LspSettings::for_worktree(server_id.as_ref(), worktree)?.settings
        {
            merge_json_value_into(user_settings, &mut settings);
        }

        Ok(Some(settings))
    }
}

zed::register_extension!(TsRsExtension);
