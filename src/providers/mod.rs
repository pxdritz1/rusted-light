use std::fmt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

use gtk4::prelude::*;

pub mod apps;
pub mod calc;
pub mod files;

pub use apps::AppsProvider;
pub use calc::CalcProvider;
pub use files::FilesProvider;

#[derive(Clone)]
pub enum SearchAction {
    LaunchApp { exec: String },
    CopyToClipboard { text: String },
    OpenFile { path: PathBuf },
    OpenFolder { path: PathBuf },
    Custom {
        description: String,
        callback: Arc<dyn Fn() + Send + Sync>,
    },
}

impl fmt::Debug for SearchAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LaunchApp { exec } => f.debug_struct("LaunchApp").field("exec", exec).finish(),
            Self::CopyToClipboard { text } => {
                f.debug_struct("CopyToClipboard").field("text", text).finish()
            }
            Self::OpenFile { path } => f.debug_struct("OpenFile").field("path", path).finish(),
            Self::OpenFolder { path } => f.debug_struct("OpenFolder").field("path", path).finish(),
            Self::Custom { description, .. } => f
                .debug_struct("Custom")
                .field("description", description)
                .finish(),
        }
    }
}

impl SearchAction {
    pub fn execute(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match self {
            SearchAction::LaunchApp { exec } => {
                let cleaned = crate::desktop_entry::clean_exec(exec);
                if !cleaned.is_empty() {
                    Command::new("sh").arg("-c").arg(cleaned).spawn()?;
                }
                Ok(())
            }
            SearchAction::CopyToClipboard { text } => {
                if let Some(display) = gtk4::gdk::Display::default() {
                    display.clipboard().set_text(text);
                    return Ok(());
                }

                copy_to_clipboard_fallback(text);
                Ok(())
            }
            SearchAction::OpenFile { path } => {
                Command::new("xdg-open").arg(path).spawn()?;
                Ok(())
            }
            SearchAction::OpenFolder { path } => {
                let folder = if path.is_dir() {
                    path.as_path()
                } else {
                    path.parent().unwrap_or(path.as_path())
                };
                Command::new("xdg-open").arg(folder).spawn()?;
                Ok(())
            }
            SearchAction::Custom { callback, .. } => {
                callback();
                Ok(())
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub icon: Option<String>,
    pub score: i64,
    pub action: SearchAction,
    pub secondary_actions: Vec<(String, SearchAction)>,
}

impl SearchResult {
    pub fn execute(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.action.execute()
    }
}

pub trait SearchProvider: Send + Sync {
    fn id(&self) -> &'static str;

    fn priority_boost(&self) -> i64 {
        0
    }

    fn matches(&self, query: &str) -> Vec<SearchResult>;
}

pub struct ProviderManager {
    providers: Vec<Box<dyn SearchProvider>>,
}

impl Default for ProviderManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderManager {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    pub fn add_provider(&mut self, provider: Box<dyn SearchProvider>) {
        self.providers.push(provider);
    }

    pub fn with_default_providers() -> Self {
        let mut manager = Self::new();
        manager.add_provider(Box::new(AppsProvider::new()));
        manager.add_provider(Box::new(CalcProvider::new()));
        manager.add_provider(Box::new(FilesProvider::new()));
        manager
    }

    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        let trimmed = query.trim();
        let mut all_results = Vec::new();

        for provider in &self.providers {
            let mut results = provider.matches(trimmed);
            let boost = provider.priority_boost();
            if boost != 0 {
                for res in &mut results {
                    res.score += boost;
                }
            }
            all_results.append(&mut results);
        }

        all_results.sort_by(|a, b| b.score.cmp(&a.score));
        all_results
    }
}

fn copy_to_clipboard_fallback(text: &str) {
    if Command::new("wl-copy").arg(text).status().is_ok() {
        return;
    }

    if let Ok(mut child) = Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(std::process::Stdio::piped())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            let _ = stdin.write_all(text.as_bytes());
        }
        return;
    }

    if let Ok(mut child) = Command::new("xsel")
        .args(["--clipboard", "--input"])
        .stdin(std::process::Stdio::piped())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            let _ = stdin.write_all(text.as_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockProvider {
        id: &'static str,
        results: Vec<SearchResult>,
    }

    impl SearchProvider for MockProvider {
        fn id(&self) -> &'static str {
            self.id
        }

        fn matches(&self, _query: &str) -> Vec<SearchResult> {
            self.results.clone()
        }
    }

    #[test]
    fn test_provider_manager_sorting() {
        let mut manager = ProviderManager::new();

        manager.add_provider(Box::new(MockProvider {
            id: "low",
            results: vec![SearchResult {
                id: "1".into(),
                title: "Low Score Item".into(),
                subtitle: None,
                icon: None,
                score: 10,
                action: SearchAction::CopyToClipboard { text: "low".into() },
                secondary_actions: Vec::new(),
            }],
        }));

        manager.add_provider(Box::new(MockProvider {
            id: "high",
            results: vec![SearchResult {
                id: "2".into(),
                title: "High Score Item".into(),
                subtitle: None,
                icon: None,
                score: 500,
                action: SearchAction::CopyToClipboard { text: "high".into() },
                secondary_actions: Vec::new(),
            }],
        }));

        let results = manager.search("anything");
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].title, "High Score Item");
        assert_eq!(results[1].title, "Low Score Item");
    }
}
