use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use std::fs;
use std::path::{Path, PathBuf};

const PLUGIN_NAME: &str = "crow";
const PLUGIN_VER: &str = env!("CARGO_PKG_VERSION");
const PLUGIN_KEY: &str = "crow@local";

// Embed all plugin files at compile time
const PLUGIN_JSON: &str = include_str!("../../plugin/.claude-plugin/plugin.json");
const CMD_STATUS: &str = include_str!("../../plugin/commands/status.md");
const CMD_REVIEW: &str = include_str!("../../plugin/commands/review.md");
const SKILL_REVIEW_PR: &str = include_str!("../../plugin/skills/review-pr/SKILL.md");
const AGENT_PR_REVIEWER: &str = include_str!("../../plugin/agents/pr-reviewer.md");

struct PluginFile {
    path: &'static str,
    content: &'static str,
}

const PLUGIN_FILES: &[PluginFile] = &[
    PluginFile {
        path: ".claude-plugin/plugin.json",
        content: PLUGIN_JSON,
    },
    PluginFile {
        path: "commands/status.md",
        content: CMD_STATUS,
    },
    PluginFile {
        path: "commands/review.md",
        content: CMD_REVIEW,
    },
    PluginFile {
        path: "skills/review-pr/SKILL.md",
        content: SKILL_REVIEW_PR,
    },
    PluginFile {
        path: "agents/pr-reviewer.md",
        content: AGENT_PR_REVIEWER,
    },
];

fn claude_dir_from(base: &Path) -> PathBuf {
    base.join(".claude")
}

fn plugin_cache_dir_from(claude: &Path) -> PathBuf {
    claude.join(format!(
        "plugins/cache/local/{}/{}",
        PLUGIN_NAME, PLUGIN_VER
    ))
}

pub fn run(uninstall: bool) -> Result<()> {
    let home = std::env::var("HOME").context("HOME not set")?;
    let base = PathBuf::from(home);
    if uninstall {
        return do_uninstall_in(&base);
    }
    do_install_in(&base)
}

fn do_install_in(base: &Path) -> Result<()> {
    let claude = claude_dir_from(base);
    let cache_dir = plugin_cache_dir_from(&claude);

    // Write all plugin files
    for file in PLUGIN_FILES {
        let dest = cache_dir.join(file.path);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory for {}", file.path))?;
        }
        fs::write(&dest, file.content).with_context(|| format!("Failed to write {}", file.path))?;
    }

    println!(
        "  {} plugin files to {}",
        "Wrote".green(),
        cache_dir.display()
    );

    // Register in installed_plugins.json
    let plugins_json_path = claude.join("plugins/installed_plugins.json");
    if plugins_json_path.exists() {
        let content = fs::read_to_string(&plugins_json_path)
            .context("Failed to read installed_plugins.json")?;
        let mut doc: serde_json::Value =
            serde_json::from_str(&content).context("Failed to parse installed_plugins.json")?;

        let now = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();
        let entry = serde_json::json!([{
            "scope": "user",
            "installPath": cache_dir.to_string_lossy(),
            "version": PLUGIN_VER,
            "installedAt": now,
            "lastUpdated": now
        }]);

        doc.as_object_mut()
            .context("installed_plugins.json is not an object")?
            .entry("plugins")
            .or_insert_with(|| serde_json::json!({}))
            .as_object_mut()
            .context("plugins field is not an object")?
            .insert(PLUGIN_KEY.to_string(), entry);

        let out = serde_json::to_string_pretty(&doc)?;
        fs::write(&plugins_json_path, out).context("Failed to write installed_plugins.json")?;
        println!("  {} in installed_plugins.json", "Registered".green());
    }

    // Enable in settings.json
    let settings_path = claude.join("settings.json");
    if settings_path.exists() {
        let content = fs::read_to_string(&settings_path).context("Failed to read settings.json")?;
        let mut doc: serde_json::Value =
            serde_json::from_str(&content).context("Failed to parse settings.json")?;

        doc.as_object_mut()
            .context("settings.json is not an object")?
            .entry("enabledPlugins")
            .or_insert_with(|| serde_json::json!({}))
            .as_object_mut()
            .context("enabledPlugins is not an object")?
            .insert(PLUGIN_KEY.to_string(), serde_json::json!(true));

        let out = serde_json::to_string_pretty(&doc)?;
        fs::write(&settings_path, out).context("Failed to write settings.json")?;
        println!("  {} in settings.json", "Enabled".green());
    }

    println!(
        "\n{} Restart Claude Code to load the crow plugin.",
        "Done.".bold()
    );
    println!("Commands: /crow:status, /crow:review");

    Ok(())
}

fn do_uninstall_in(base: &Path) -> Result<()> {
    let claude = claude_dir_from(base);
    let cache_dir = plugin_cache_dir_from(&claude);

    // Remove plugin files
    if cache_dir.exists() {
        fs::remove_dir_all(&cache_dir).context("Failed to remove plugin cache")?;
        println!("  {} {}", "Removed".yellow(), cache_dir.display());
    }

    // Deregister from installed_plugins.json
    let plugins_json_path = claude.join("plugins/installed_plugins.json");
    if plugins_json_path.exists() {
        let content = fs::read_to_string(&plugins_json_path)?;
        let mut doc: serde_json::Value = serde_json::from_str(&content)?;
        if let Some(plugins) = doc.get_mut("plugins").and_then(|v| v.as_object_mut()) {
            plugins.remove(PLUGIN_KEY);
        }
        fs::write(&plugins_json_path, serde_json::to_string_pretty(&doc)?)?;
        println!("  {} from installed_plugins.json", "Removed".yellow());
    }

    // Disable in settings.json
    let settings_path = claude.join("settings.json");
    if settings_path.exists() {
        let content = fs::read_to_string(&settings_path)?;
        let mut doc: serde_json::Value = serde_json::from_str(&content)?;
        if let Some(enabled) = doc
            .get_mut("enabledPlugins")
            .and_then(|v| v.as_object_mut())
        {
            enabled.remove(PLUGIN_KEY);
        }
        fs::write(&settings_path, serde_json::to_string_pretty(&doc)?)?;
        println!("  {} in settings.json", "Disabled".yellow());
    }

    println!("\n{} Restart Claude Code to apply.", "Done.".bold());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn cache_dir_for(base: &std::path::Path) -> PathBuf {
        claude_dir_from(base)
            .join("plugins/cache/local")
            .join(PLUGIN_NAME)
            .join(PLUGIN_VER)
    }

    // --- do_install_in ---

    #[test]
    fn do_install_in_creates_all_plugin_files() {
        let tmp = tempfile::tempdir().expect("failed to create tempdir");
        do_install_in(tmp.path()).unwrap();

        let cache_dir = cache_dir_for(tmp.path());
        assert!(cache_dir.join(".claude-plugin/plugin.json").exists());
        assert!(cache_dir.join("commands/status.md").exists());
        assert!(cache_dir.join("commands/review.md").exists());
        assert!(cache_dir.join("skills/review-pr/SKILL.md").exists());
        assert!(cache_dir.join("agents/pr-reviewer.md").exists());
    }

    #[test]
    fn do_install_in_does_not_create_deleted_command_files() {
        let tmp = tempfile::tempdir().expect("failed to create tempdir");
        do_install_in(tmp.path()).unwrap();

        let cache_dir = cache_dir_for(tmp.path());
        assert!(!cache_dir.join("commands/checkout.md").exists());
        assert!(!cache_dir.join("commands/reviews.md").exists());
        assert!(!cache_dir.join("commands/ci.md").exists());
        assert!(!cache_dir.join("commands/push.md").exists());
        assert!(!cache_dir.join("commands/done.md").exists());
        assert!(!cache_dir.join("commands/comment.md").exists());
    }

    #[test]
    fn do_install_in_writes_valid_plugin_json() {
        let tmp = tempfile::tempdir().expect("failed to create tempdir");
        do_install_in(tmp.path()).unwrap();

        let cache_dir = cache_dir_for(tmp.path());
        let content = fs::read_to_string(cache_dir.join(".claude-plugin/plugin.json")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(parsed.is_object());
    }

    #[test]
    fn do_install_in_skips_plugins_json_when_absent() {
        // Without a pre-existing installed_plugins.json, the install should still succeed
        let tmp = tempfile::tempdir().expect("failed to create tempdir");
        do_install_in(tmp.path()).unwrap();

        // The installed_plugins.json should NOT have been created
        let plugins_json = tmp.path().join(".claude/plugins/installed_plugins.json");
        assert!(!plugins_json.exists());
    }

    #[test]
    fn do_install_in_registers_in_existing_plugins_json() {
        let tmp = tempfile::tempdir().expect("failed to create tempdir");
        let claude = tmp.path().join(".claude");
        let plugins_dir = claude.join("plugins");
        fs::create_dir_all(&plugins_dir).unwrap();
        let plugins_json_path = plugins_dir.join("installed_plugins.json");
        fs::write(&plugins_json_path, r#"{"plugins":{}}"#).unwrap();

        do_install_in(tmp.path()).unwrap();

        let content = fs::read_to_string(&plugins_json_path).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(doc["plugins"][PLUGIN_KEY].is_array());
    }

    #[test]
    fn do_install_in_skips_settings_json_when_absent() {
        let tmp = tempfile::tempdir().expect("failed to create tempdir");
        do_install_in(tmp.path()).unwrap();

        let settings_path = tmp.path().join(".claude/settings.json");
        assert!(!settings_path.exists());
    }

    #[test]
    fn do_install_in_enables_plugin_in_existing_settings_json() {
        let tmp = tempfile::tempdir().expect("failed to create tempdir");
        let claude = tmp.path().join(".claude");
        fs::create_dir_all(&claude).unwrap();
        let settings_path = claude.join("settings.json");
        fs::write(&settings_path, r#"{"enabledPlugins":{}}"#).unwrap();

        do_install_in(tmp.path()).unwrap();

        let content = fs::read_to_string(&settings_path).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(doc["enabledPlugins"][PLUGIN_KEY], serde_json::json!(true));
    }

    // --- do_uninstall_in ---

    #[test]
    fn do_uninstall_in_removes_existing_cache_dir() {
        let tmp = tempfile::tempdir().expect("failed to create tempdir");

        // Install first
        do_install_in(tmp.path()).unwrap();
        let cache_dir = cache_dir_for(tmp.path());
        assert!(cache_dir.exists());

        // Uninstall
        do_uninstall_in(tmp.path()).unwrap();
        assert!(!cache_dir.exists());
    }

    #[test]
    fn do_uninstall_in_is_noop_when_cache_absent() {
        let tmp = tempfile::tempdir().expect("failed to create tempdir");
        // No install — should succeed silently
        do_uninstall_in(tmp.path()).unwrap();
    }

    #[test]
    fn do_uninstall_in_removes_from_plugins_json() {
        let tmp = tempfile::tempdir().expect("failed to create tempdir");
        let claude = tmp.path().join(".claude");
        let plugins_dir = claude.join("plugins");
        fs::create_dir_all(&plugins_dir).unwrap();
        let plugins_json_path = plugins_dir.join("installed_plugins.json");
        let initial = serde_json::json!({
            "plugins": {
                PLUGIN_KEY: [{"scope": "user"}]
            }
        });
        fs::write(
            &plugins_json_path,
            serde_json::to_string_pretty(&initial).unwrap(),
        )
        .unwrap();

        do_uninstall_in(tmp.path()).unwrap();

        let content = fs::read_to_string(&plugins_json_path).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(doc["plugins"].get(PLUGIN_KEY).is_none());
    }

    #[test]
    fn do_uninstall_in_disables_in_settings_json() {
        let tmp = tempfile::tempdir().expect("failed to create tempdir");
        let claude = tmp.path().join(".claude");
        fs::create_dir_all(&claude).unwrap();
        let settings_path = claude.join("settings.json");
        let initial = serde_json::json!({
            "enabledPlugins": {
                PLUGIN_KEY: true
            }
        });
        fs::write(
            &settings_path,
            serde_json::to_string_pretty(&initial).unwrap(),
        )
        .unwrap();

        do_uninstall_in(tmp.path()).unwrap();

        let content = fs::read_to_string(&settings_path).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(doc["enabledPlugins"].get(PLUGIN_KEY).is_none());
    }

    // --- helper accessors ---

    #[test]
    fn claude_dir_from_appends_claude() {
        let base = PathBuf::from("/tmp/test-home");
        let dir = claude_dir_from(&base);
        assert_eq!(dir, PathBuf::from("/tmp/test-home/.claude"));
    }

    #[test]
    fn plugin_cache_dir_from_contains_name_and_version() {
        let claude = PathBuf::from("/tmp/.claude");
        let dir = plugin_cache_dir_from(&claude);
        assert!(dir.to_string_lossy().contains(PLUGIN_NAME));
        assert!(dir.to_string_lossy().contains(PLUGIN_VER));
    }

    #[test]
    fn uninstall_from_nonexistent_dir_is_noop() {
        let tmp = tempfile::tempdir().expect("failed to create tempdir");
        // Never installed — should succeed without error
        do_uninstall_in(tmp.path()).unwrap();
    }
}
