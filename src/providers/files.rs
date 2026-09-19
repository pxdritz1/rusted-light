use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;

use ignore::WalkBuilder;
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

use super::{SearchAction, SearchProvider, SearchResult};

#[derive(Clone, Debug)]
pub struct FileEntry {
    pub path: PathBuf,
    pub file_name: String,
    pub is_dir: bool,
}

pub struct FilesProvider {
    entries: Arc<RwLock<Vec<FileEntry>>>,
    is_indexing: Arc<AtomicBool>,
    home_dir: Option<PathBuf>,
}

impl Default for FilesProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl FilesProvider {
    pub fn new() -> Self {
        let home_dir = dirs::home_dir().or_else(|| std::env::var("HOME").ok().map(PathBuf::from));
        let entries = Arc::new(RwLock::new(Vec::new()));
        let is_indexing = Arc::new(AtomicBool::new(false));

        let provider = Self {
            entries: entries.clone(),
            is_indexing: is_indexing.clone(),
            home_dir: home_dir.clone(),
        };

        if let Some(home) = home_dir {
            provider.start_background_indexing(home, entries, is_indexing);
        }

        provider
    }

    pub fn is_indexing(&self) -> bool {
        self.is_indexing.load(Ordering::Acquire)
    }

    pub fn entry_count(&self) -> usize {
        self.entries.read().map(|e| e.len()).unwrap_or(0)
    }

    fn start_background_indexing(
        &self,
        home: PathBuf,
        entries: Arc<RwLock<Vec<FileEntry>>>,
        is_indexing: Arc<AtomicBool>,
    ) {
        thread::Builder::new()
            .name("rusted-files-indexer".into())
            .spawn(move || {
                is_indexing.store(true, Ordering::Release);

                let mut builder = WalkBuilder::new(&home);
                builder
                    .hidden(true)
                    .git_ignore(true)
                    .filter_entry(|entry| {
                        let name = entry.file_name().to_string_lossy();
                        !matches!(
                            name.as_ref(),
                            "node_modules"
                                | "target"
                                | ".git"
                                | ".cache"
                                | ".cargo"
                                | ".rustup"
                                | ".npm"
                                | ".venv"
                                | "vendor"
                        )
                    });

                let walker = builder.build();
                let mut local_entries = Vec::new();
                const MAX_INDEXED_FILES: usize = 75_000;

                for result in walker {
                    let Ok(entry) = result else {
                        continue;
                    };

                    let path = entry.path().to_path_buf();
                    if path == home {
                        continue;
                    }

                    let file_name = match entry.file_name().to_str() {
                        Some(name) => name.to_string(),
                        None => continue,
                    };

                    let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);

                    local_entries.push(FileEntry {
                        path,
                        file_name,
                        is_dir,
                    });

                    if local_entries.len() >= MAX_INDEXED_FILES {
                        break;
                    }
                }

                if let Ok(mut lock) = entries.write() {
                    *lock = local_entries;
                }

                is_indexing.store(false, Ordering::Release);
            })
            .expect("Failed to start file indexer thread");
    }

    fn prettify_path(&self, path: &Path) -> String {
        if let Some(ref home) = self.home_dir {
            if let Ok(suffix) = path.strip_prefix(home) {
                return format!("~/{}", suffix.display());
            }
        }
        path.display().to_string()
    }

    fn search_plocate(&self, query: &str) -> Vec<SearchResult> {
        let output = Command::new("plocate")
            .args(["-l", "15", "-i", query])
            .output();

        let Ok(out) = output else {
            return Vec::new();
        };

        if !out.status.success() {
            return Vec::new();
        }

        let text = String::from_utf8_lossy(&out.stdout);
        let mut results = Vec::new();

        for (idx, line) in text.lines().enumerate() {
            let path = PathBuf::from(line.trim());
            if !path.exists() {
                continue;
            }

            let file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| line.to_string());

            let is_dir = path.is_dir();
            let subtitle = self.prettify_path(&path);

            let icon = if is_dir {
                Some("folder".to_string())
            } else {
                icon_for_filename(&file_name)
            };

            results.push(SearchResult {
                id: format!("file:{}", path.display()),
                title: file_name,
                subtitle: Some(subtitle),
                icon,
                score: 100 - (idx as i64),
                action: SearchAction::OpenFile { path: path.clone() },
                secondary_actions: vec![(
                    "Open folder".to_string(),
                    SearchAction::OpenFolder { path },
                )],
            });
        }

        results
    }
}

impl SearchProvider for FilesProvider {
    fn id(&self) -> &'static str {
        "files"
    }

    fn priority_boost(&self) -> i64 {
        0
    }

    fn matches(&self, query: &str) -> Vec<SearchResult> {
        let q = query.trim();
        if q.len() < 2 {
            return Vec::new();
        }

        let entries_guard = match self.entries.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if entries_guard.is_empty() {
            return self.search_plocate(q);
        }

        let pattern = Pattern::parse(q, CaseMatching::Smart, Normalization::Smart);
        let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
        let mut name_buf = Vec::new();
        let mut path_buf = Vec::new();

        let mut scored_results = Vec::new();

        for entry in entries_guard.iter() {
            let name_utf32 = Utf32Str::new(&entry.file_name, &mut name_buf);
            let name_score = pattern.score(name_utf32, &mut matcher);

            let score = if let Some(score) = name_score {
                (score as i64) * 2
            } else {
                let path_str = entry.path.to_string_lossy();
                let path_utf32 = Utf32Str::new(&path_str, &mut path_buf);
                if let Some(path_score) = pattern.score(path_utf32, &mut matcher) {
                    path_score as i64
                } else {
                    continue;
                }
            };

            scored_results.push((score, entry));
        }

        scored_results.sort_by(|(s1, _), (s2, _)| s2.cmp(s1));

        scored_results
            .into_iter()
            .take(15)
            .map(|(score, entry)| {
                let icon = if entry.is_dir {
                    Some("folder".to_string())
                } else {
                    icon_for_filename(&entry.file_name)
                };

                let subtitle = self.prettify_path(&entry.path);

                SearchResult {
                    id: format!("file:{}", entry.path.display()),
                    title: entry.file_name.clone(),
                    subtitle: Some(subtitle),
                    icon,
                    score,
                    action: SearchAction::OpenFile {
                        path: entry.path.clone(),
                    },
                    secondary_actions: vec![(
                        "Open folder".to_string(),
                        SearchAction::OpenFolder {
                            path: entry.path.clone(),
                        },
                    )],
                }
            })
            .collect()
    }
}

fn icon_for_filename(name: &str) -> Option<String> {
    let lower = name.to_lowercase();
    let ext = lower.rsplit('.').next()?;

    let icon_name = match ext {
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" => "image-x-generic",
        "mp4" | "mkv" | "webm" | "avi" | "mov" => "video-x-generic",
        "mp3" | "flac" | "wav" | "ogg" | "m4a" => "audio-x-generic",
        "pdf" => "application-pdf",
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" => "package-x-generic",
        "rs" | "py" | "js" | "ts" | "c" | "cpp" | "h" | "go" | "html" | "css" | "json"
        | "toml" | "yaml" | "yml" | "sh" => "text-x-script",
        "txt" | "md" => "text-x-generic",
        _ => "text-x-generic",
    };

    Some(icon_name.to_string())
}
