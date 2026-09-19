use std::sync::RwLock;

use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

use crate::desktop_entry::{scan_applications, AppEntry};

use super::{SearchAction, SearchProvider, SearchResult};

pub struct AppsProvider {
    apps: RwLock<Vec<AppEntry>>,
}

impl Default for AppsProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AppsProvider {
    pub fn new() -> Self {
        let apps = scan_applications();
        Self {
            apps: RwLock::new(apps),
        }
    }

    pub fn reload(&self) {
        let reloaded = scan_applications();
        if let Ok(mut lock) = self.apps.write() {
            *lock = reloaded;
        }
    }
}

impl SearchProvider for AppsProvider {
    fn id(&self) -> &'static str {
        "apps"
    }

    fn priority_boost(&self) -> i64 {
        50
    }

    fn matches(&self, query: &str) -> Vec<SearchResult> {
        let apps_guard = match self.apps.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if query.trim().is_empty() {
            return apps_guard
                .iter()
                .map(|app| SearchResult {
                    id: format!("app:{}", app.name),
                    title: app.name.clone(),
                    subtitle: Some(app.exec.clone()),
                    icon: app.icon.clone(),
                    score: 0,
                    action: SearchAction::LaunchApp {
                        exec: app.exec.clone(),
                    },
                    secondary_actions: Vec::new(),
                })
                .collect();
        }

        let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);
        let mut matcher = Matcher::new(Config::DEFAULT);
        let mut name_buf = Vec::new();
        let mut exec_buf = Vec::new();

        let mut results = Vec::new();

        for app in apps_guard.iter() {
            let name_utf32 = Utf32Str::new(&app.name, &mut name_buf);
            let name_score = pattern.score(name_utf32, &mut matcher);

            let score = if let Some(score) = name_score {
                score as i64
            } else {
                let exec_utf32 = Utf32Str::new(&app.exec, &mut exec_buf);
                if let Some(exec_score) = pattern.score(exec_utf32, &mut matcher) {
                    (exec_score as i64 * 7) / 10
                } else {
                    continue;
                }
            };

            results.push(SearchResult {
                id: format!("app:{}", app.name),
                title: app.name.clone(),
                subtitle: Some(app.exec.clone()),
                icon: app.icon.clone(),
                score,
                action: SearchAction::LaunchApp {
                    exec: app.exec.clone(),
                },
                secondary_actions: Vec::new(),
            });
        }

        results.sort_by(|a, b| b.score.cmp(&a.score));
        results
    }
}
