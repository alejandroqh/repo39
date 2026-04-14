use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Write};
use std::path::Path;

// Dirs whose existence is a signal but whose contents are noise for scoring.
const SKIP_INTERIOR: &[&str] = &["node_modules", ".git", "target", "__pycache__", "vendor"];

enum Marker {
    File(&'static str, f64),
    Dir(&'static str, f64),
    Path(&'static str, f64),
    Ext(&'static str, f64),
}

struct ScanData {
    root_files: HashSet<String>,
    root_dirs: HashSet<String>,
    depth1_paths: HashSet<String>,
    ext_counts: HashMap<String, usize>,
}

fn scan(root: &Path) -> io::Result<ScanData> {
    let mut data = ScanData {
        root_files: HashSet::new(),
        root_dirs: HashSet::new(),
        depth1_paths: HashSet::new(),
        ext_counts: HashMap::new(),
    };

    for entry in fs::read_dir(root)?.filter_map(|e| e.ok()) {
        let name = entry.file_name();
        let name_str = match name.to_str() {
            Some(s) => s.to_string(),
            None => continue,
        };

        let ft = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };

        if ft.is_file() {
            count_ext(&name_str, &mut data.ext_counts);
            data.root_files.insert(name_str);
        } else if ft.is_dir() {
            data.root_dirs.insert(name_str.clone());
            if !SKIP_INTERIOR.contains(&name_str.as_str()) {
                scan_depth1(root, &name_str, &mut data)?;
            }
        }
    }

    Ok(data)
}

fn scan_depth1(root: &Path, dir_name: &str, data: &mut ScanData) -> io::Result<()> {
    let dir_path = root.join(dir_name);
    let entries = match fs::read_dir(&dir_path) {
        Ok(rd) => rd,
        Err(_) => return Ok(()),
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let name = entry.file_name();
        let name_str = match name.to_str() {
            Some(s) => s.to_string(),
            None => continue,
        };

        let ft = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };

        let rel = format!("{dir_name}/{name_str}");
        data.depth1_paths.insert(rel);

        if ft.is_file() {
            count_ext(&name_str, &mut data.ext_counts);
        }
    }

    Ok(())
}

// Fingerprint extensions are ASCII lowercase; match with ascii_lowercase.
fn count_ext(name: &str, counts: &mut HashMap<String, usize>) {
    if let Some((prefix, ext)) = name.rsplit_once('.') {
        if !prefix.is_empty() && !ext.is_empty() {
            *counts.entry(ext.to_ascii_lowercase()).or_default() += 1;
        }
    }
}

fn score_marker(marker: &Marker, data: &ScanData) -> f64 {
    match marker {
        Marker::File(name, w) => {
            if data.root_files.contains(*name) { *w } else { 0.0 }
        }
        Marker::Dir(name, w) => {
            if data.root_dirs.contains(*name) { *w } else { 0.0 }
        }
        Marker::Path(path, w) => {
            if data.depth1_paths.contains(*path) { *w } else { 0.0 }
        }
        Marker::Ext(ext, w) => {
            if let Some(&count) = data.ext_counts.get(*ext) {
                w * (count as f64 / 3.0).min(1.0)
            } else {
                0.0
            }
        }
    }
}

fn score_all(data: &ScanData) -> Vec<(&'static str, f64)> {
    let db = fingerprints();
    let mut results: Vec<(&str, f64)> = db
        .iter()
        .map(|(name, markers)| {
            let raw: f64 = markers.iter().map(|m| score_marker(m, data)).sum();
            (*name, raw.min(1.0))
        })
        .filter(|(_, c)| *c > 0.0)
        .collect();

    results.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(b.0))
    });
    results.truncate(5);
    results
}

pub fn run_identify(root: &Path, out: &mut impl Write) -> io::Result<()> {
    let data = scan(root)?;
    let results = score_all(&data);
    for (name, confidence) in &results {
        writeln!(out, "{name} {confidence:.2}")?;
    }
    Ok(())
}

fn fingerprints() -> &'static [(&'static str, &'static [Marker])] {
    use Marker::*;
    &[
        // ── Languages (22) ──────────────────────────────────────────────
        ("rust", &[
            File("Cargo.toml", 0.45),
            File("Cargo.lock", 0.15),
            Path("src/main.rs", 0.10),
            Path("src/lib.rs", 0.10),
            Ext("rs", 0.15),
            File("rust-toolchain.toml", 0.05),
            File("rustfmt.toml", 0.03),
            Dir(".cargo", 0.02),
        ]),
        ("javascript", &[
            File("package.json", 0.35),
            Ext("js", 0.30),
            Ext("mjs", 0.10),
            File("jsconfig.json", 0.10),
            File(".eslintrc.json", 0.05),
            Dir("node_modules", 0.05),
            File("webpack.config.js", 0.05),
        ]),
        ("typescript", &[
            File("tsconfig.json", 0.45),
            Ext("ts", 0.30),
            Ext("tsx", 0.15),
            File("package.json", 0.05),
            File("tslint.json", 0.05),
        ]),
        ("python", &[
            File("pyproject.toml", 0.30),
            File("setup.py", 0.25),
            File("requirements.txt", 0.20),
            Ext("py", 0.30),
            File("Pipfile", 0.10),
            File("poetry.lock", 0.08),
            File(".python-version", 0.05),
            Dir("__pycache__", 0.05),
            Dir("venv", 0.03),
            Dir(".venv", 0.03),
        ]),
        ("java", &[
            File("pom.xml", 0.40),
            File("build.gradle", 0.40),
            Ext("java", 0.30),
            Path("src/main", 0.10),
            File("gradlew", 0.08),
            File("mvnw", 0.08),
        ]),
        ("kotlin", &[
            Ext("kt", 0.35),
            File("build.gradle.kts", 0.30),
            File("settings.gradle.kts", 0.15),
            Ext("kts", 0.10),
            File("gradlew", 0.08),
        ]),
        ("go", &[
            File("go.mod", 0.50),
            File("go.sum", 0.15),
            Ext("go", 0.30),
            Dir("cmd", 0.08),
            Dir("pkg", 0.05),
            Dir("internal", 0.05),
        ]),
        ("swift", &[
            File("Package.swift", 0.45),
            Ext("swift", 0.35),
            Dir("Sources", 0.10),
            Dir("Tests", 0.05),
            File(".swiftlint.yml", 0.05),
        ]),
        ("ruby", &[
            File("Gemfile", 0.40),
            Ext("rb", 0.30),
            File("Rakefile", 0.10),
            File(".ruby-version", 0.08),
            File("Gemfile.lock", 0.08),
            Dir("spec", 0.05),
        ]),
        ("php", &[
            File("composer.json", 0.45),
            Ext("php", 0.35),
            File("composer.lock", 0.08),
            File("artisan", 0.10),
            Dir("vendor", 0.05),
        ]),
        ("c", &[
            Ext("c", 0.35),
            Ext("h", 0.25),
            File("Makefile", 0.12),
            File("CMakeLists.txt", 0.20),
            File("configure", 0.10),
        ]),
        ("cpp", &[
            Ext("cpp", 0.30),
            Ext("hpp", 0.15),
            Ext("cc", 0.12),
            File("CMakeLists.txt", 0.18),
            File("Makefile", 0.08),
            File("conanfile.txt", 0.08),
            File("vcpkg.json", 0.08),
        ]),
        ("csharp", &[
            Ext("cs", 0.35),
            Ext("sln", 0.25),
            Ext("csproj", 0.20),
            File("global.json", 0.08),
            File("nuget.config", 0.05),
        ]),
        ("dart", &[
            File("pubspec.yaml", 0.50),
            Ext("dart", 0.30),
            Dir("lib", 0.08),
            File("pubspec.lock", 0.05),
            File("analysis_options.yaml", 0.05),
        ]),
        ("elixir", &[
            File("mix.exs", 0.50),
            Ext("ex", 0.25),
            Ext("exs", 0.12),
            Dir("lib", 0.08),
            File("mix.lock", 0.05),
        ]),
        ("zig", &[
            File("build.zig", 0.50),
            Ext("zig", 0.35),
            File("build.zig.zon", 0.12),
            Dir("src", 0.05),
        ]),
        ("haskell", &[
            File("stack.yaml", 0.30),
            Ext("hs", 0.35),
            Ext("cabal", 0.20),
            File("Setup.hs", 0.08),
            Dir("app", 0.05),
        ]),
        ("scala", &[
            File("build.sbt", 0.45),
            Ext("scala", 0.30),
            Dir("project", 0.10),
            Ext("sc", 0.08),
        ]),
        ("lua", &[
            Ext("lua", 0.45),
            File("init.lua", 0.18),
            File(".luacheckrc", 0.12),
            Dir("lua", 0.08),
        ]),
        ("perl", &[
            Ext("pl", 0.30),
            Ext("pm", 0.20),
            File("Makefile.PL", 0.18),
            File("cpanfile", 0.15),
            Dir("t", 0.08),
        ]),
        ("r", &[
            File("DESCRIPTION", 0.25),
            Ext("R", 0.30),
            Ext("Rmd", 0.12),
            File("NAMESPACE", 0.15),
            Dir("R", 0.08),
            File(".Rprofile", 0.08),
        ]),
        ("shell", &[
            Ext("sh", 0.35),
            Ext("bash", 0.18),
            Ext("zsh", 0.12),
            Dir("bin", 0.08),
            Dir("scripts", 0.08),
            File("Makefile", 0.05),
        ]),

        // ── Frameworks (18) ────────────────────────────────────────────
        ("react", &[
            Ext("jsx", 0.35),
            Ext("tsx", 0.20),
            Path("src/App.jsx", 0.15),
            Path("src/App.tsx", 0.15),
            Dir("public", 0.08),
            File("package.json", 0.05),
        ]),
        ("vuejs", &[
            Ext("vue", 0.45),
            Path("src/App.vue", 0.18),
            File("vue.config.js", 0.15),
            File("vite.config.ts", 0.08),
            File("package.json", 0.05),
        ]),
        ("nextjs", &[
            File("next.config.js", 0.40),
            File("next.config.mjs", 0.40),
            File("next.config.ts", 0.40),
            Dir("pages", 0.15),
            Dir("app", 0.10),
            Dir(".next", 0.08),
        ]),
        ("nuxtjs", &[
            File("nuxt.config.ts", 0.45),
            File("nuxt.config.js", 0.45),
            Dir("pages", 0.12),
            Dir("layouts", 0.08),
            Dir(".nuxt", 0.08),
        ]),
        ("angular", &[
            File("angular.json", 0.50),
            File("tsconfig.app.json", 0.12),
            Path("src/main.ts", 0.08),
            Dir("e2e", 0.08),
            File("package.json", 0.05),
        ]),
        ("svelte", &[
            File("svelte.config.js", 0.40),
            File("svelte.config.ts", 0.40),
            Ext("svelte", 0.35),
            Path("src/app.html", 0.08),
            Dir("routes", 0.08),
        ]),
        ("django", &[
            File("manage.py", 0.40),
            Dir("templates", 0.15),
            File("urls.py", 0.10),
            File("wsgi.py", 0.08),
            File("settings.py", 0.12),
            Ext("py", 0.08),
        ]),
        ("flask", &[
            File("app.py", 0.25),
            Dir("templates", 0.18),
            Dir("static", 0.12),
            File("requirements.txt", 0.08),
            Ext("py", 0.12),
        ]),
        ("fastapi", &[
            File("main.py", 0.20),
            Dir("routers", 0.18),
            File("alembic.ini", 0.12),
            Dir("app", 0.10),
            Ext("py", 0.12),
            File("requirements.txt", 0.05),
        ]),
        ("rails", &[
            File("config.ru", 0.20),
            File("Gemfile", 0.15),
            Dir("app", 0.12),
            Dir("db", 0.12),
            Dir("config", 0.08),
            Path("config/routes.rb", 0.12),
            Ext("rb", 0.08),
        ]),
        ("spring", &[
            File("application.properties", 0.20),
            File("application.yml", 0.20),
            File("pom.xml", 0.15),
            Path("src/main", 0.12),
            Ext("java", 0.10),
            File("gradlew", 0.05),
        ]),
        ("flutter", &[
            File("pubspec.yaml", 0.25),
            Dir("android", 0.15),
            Dir("ios", 0.15),
            Ext("dart", 0.20),
            Path("lib/main.dart", 0.12),
            File(".flutter-plugins", 0.08),
        ]),
        ("tauri", &[
            Dir("src-tauri", 0.50),
            File("tauri.conf.json", 0.15),
            File("Cargo.toml", 0.08),
            File("package.json", 0.05),
            Ext("ts", 0.05),
        ]),
        ("electron", &[
            File("electron-builder.yml", 0.30),
            File("forge.config.js", 0.25),
            File("electron.config.js", 0.12),
            Path("src/preload.js", 0.12),
            Path("src/preload.ts", 0.12),
            File("package.json", 0.05),
        ]),
        ("express", &[
            Dir("routes", 0.18),
            File("app.js", 0.18),
            File("server.js", 0.12),
            Dir("middleware", 0.12),
            Dir("controllers", 0.10),
            File("package.json", 0.05),
        ]),
        ("nestjs", &[
            File("nest-cli.json", 0.45),
            Path("src/app.module.ts", 0.18),
            File("tsconfig.build.json", 0.08),
            Path("src/main.ts", 0.08),
            File("package.json", 0.05),
        ]),
        ("remix", &[
            File("remix.config.js", 0.40),
            File("remix.config.ts", 0.40),
            Path("app/root.tsx", 0.18),
            Path("app/entry.server.tsx", 0.12),
            Dir("routes", 0.08),
        ]),
        ("astro", &[
            File("astro.config.mjs", 0.40),
            File("astro.config.ts", 0.40),
            Ext("astro", 0.30),
            Path("src/pages", 0.10),
            Dir("public", 0.05),
        ]),

        // ── Non-code (12) ──────────────────────────────────────────────
        ("docs", &[
            Dir("docs", 0.30),
            File("mkdocs.yml", 0.22),
            File("docusaurus.config.js", 0.22),
            File("book.toml", 0.18),
            Ext("md", 0.18),
            Ext("rst", 0.15),
        ]),
        ("markdown", &[
            Ext("md", 0.40),
            Ext("mdx", 0.18),
            File("README.md", 0.12),
            File("CONTRIBUTING.md", 0.08),
            File("CHANGELOG.md", 0.08),
        ]),
        ("images", &[
            Ext("png", 0.18),
            Ext("jpg", 0.18),
            Ext("jpeg", 0.12),
            Ext("svg", 0.12),
            Ext("gif", 0.08),
            Ext("webp", 0.08),
            Dir("images", 0.10),
            Dir("assets", 0.08),
        ]),
        ("video", &[
            Ext("mp4", 0.25),
            Ext("mkv", 0.18),
            Ext("mov", 0.18),
            Ext("avi", 0.12),
            Ext("webm", 0.12),
            Dir("videos", 0.08),
        ]),
        ("audio", &[
            Ext("mp3", 0.25),
            Ext("wav", 0.18),
            Ext("flac", 0.18),
            Ext("ogg", 0.12),
            Ext("aac", 0.10),
            Dir("audio", 0.08),
        ]),
        ("data", &[
            Ext("csv", 0.18),
            Ext("sql", 0.15),
            Ext("json", 0.12),
            Ext("xml", 0.10),
            Ext("parquet", 0.12),
            Ext("sqlite", 0.10),
            Dir("data", 0.12),
            Dir("datasets", 0.08),
        ]),
        ("config", &[
            File("Dockerfile", 0.15),
            File("docker-compose.yml", 0.12),
            File("docker-compose.yaml", 0.12),
            File(".editorconfig", 0.08),
            File(".gitignore", 0.08),
            Ext("env", 0.05),
            Ext("toml", 0.08),
            Ext("yaml", 0.08),
        ]),
        ("devops", &[
            Dir(".github", 0.15),
            File("Dockerfile", 0.15),
            File("docker-compose.yml", 0.12),
            File("Jenkinsfile", 0.15),
            File(".gitlab-ci.yml", 0.12),
            Ext("tf", 0.12),
            Dir("terraform", 0.12),
            Dir("k8s", 0.10),
            Dir("helm", 0.08),
        ]),
        ("monorepo", &[
            File("lerna.json", 0.30),
            File("pnpm-workspace.yaml", 0.30),
            File("turbo.json", 0.30),
            File("nx.json", 0.30),
            Dir("packages", 0.20),
            Dir("apps", 0.15),
        ]),
        ("latex", &[
            Ext("tex", 0.40),
            Ext("bib", 0.12),
            Ext("sty", 0.08),
            File("latexmkrc", 0.12),
            File(".latexmkrc", 0.12),
            Dir("figures", 0.08),
            Dir("chapters", 0.08),
        ]),
        ("design", &[
            Ext("psd", 0.18),
            Ext("sketch", 0.15),
            Ext("fig", 0.12),
            Ext("xd", 0.12),
            Ext("ai", 0.12),
            Dir("design", 0.12),
            Dir("mockups", 0.10),
            Ext("eps", 0.08),
        ]),
        ("gamedev", &[
            File("project.godot", 0.30),
            Ext("gd", 0.20),
            Ext("tscn", 0.15),
            Dir("Assets", 0.15),
            Ext("unity", 0.12),
            Ext("wgsl", 0.10),
            Ext("glsl", 0.10),
            Dir("addons", 0.08),
        ]),
    ]
}
