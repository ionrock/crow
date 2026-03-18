use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use std::fs;
use std::path::PathBuf;

const PLUGIN_NAME: &str = "crow";
const PLUGIN_VER: &str = env!("CARGO_PKG_VERSION");
const PLUGIN_KEY: &str = "crow@local";

// Embed all plugin files at compile time
const PLUGIN_JSON: &str = include_str!("../../plugin/.claude-plugin/plugin.json");
const CMD_STATUS: &str = include_str!("../../plugin/commands/status.md");
const CMD_CHECKOUT: &str = include_str!("../../plugin/commands/checkout.md");
const CMD_REVIEWS: &str = include_str!("../../plugin/commands/reviews.md");
const CMD_CI: &str = include_str!("../../plugin/commands/ci.md");
const CMD_PUSH: &str = include_str!("../../plugin/commands/push.md");
const CMD_DONE: &str = include_str!("../../plugin/commands/done.md");
const CMD_COMMENT: &str = include_str!("../../plugin/commands/comment.md");
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
        path: "commands/checkout.md",
        content: CMD_CHECKOUT,
    },
    PluginFile {
        path: "commands/reviews.md",
        content: CMD_REVIEWS,
    },
    PluginFile {
        path: "commands/ci.md",
        content: CMD_CI,
    },
    PluginFile {
        path: "commands/push.md",
        content: CMD_PUSH,
    },
    PluginFile {
        path: "commands/done.md",
        content: CMD_DONE,
    },
    PluginFile {
        path: "commands/comment.md",
        content: CMD_COMMENT,
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

fn claude_dir() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME not set")?;
    Ok(PathBuf::from(home).join(".claude"))
}

fn plugin_cache_dir() -> Result<PathBuf> {
    Ok(claude_dir()?.join(format!(
        "plugins/cache/local/{}/{}",
        PLUGIN_NAME, PLUGIN_VER
    )))
}

pub fn run(uninstall: bool) -> Result<()> {
    if uninstall {
        return do_uninstall();
    }
    do_install()
}

fn do_install() -> Result<()> {
    let cache_dir = plugin_cache_dir()?;

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
    let plugins_json_path = claude_dir()?.join("plugins/installed_plugins.json");
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
    let settings_path = claude_dir()?.join("settings.json");
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
    println!("Commands: /crow:status, /crow:reviews, /crow:ci, /crow:checkout, etc.");

    Ok(())
}

fn do_uninstall() -> Result<()> {
    let cache_dir = plugin_cache_dir()?;

    // Remove plugin files
    if cache_dir.exists() {
        fs::remove_dir_all(&cache_dir).context("Failed to remove plugin cache")?;
        println!("  {} {}", "Removed".yellow(), cache_dir.display());
    }

    // Deregister from installed_plugins.json
    let plugins_json_path = claude_dir()?.join("plugins/installed_plugins.json");
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
    let settings_path = claude_dir()?.join("settings.json");
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
