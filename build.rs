use std::process::Command;
use std::fs;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    let frontend_dir = Path::new("frontend");
    let path = std::env::var("PATH").unwrap_or_default();

    println!("PATH: {path}");

    let npm_exe = if cfg!(windows) {
        "npm.cmd"
    } else {
        "npm"
    };

    // 1. Собираем зависимости и билдим фронтенд
    println!("Building frontend...");
    let status = Command::new(npm_exe)
        .arg("install")
        .current_dir(frontend_dir)
        .env("PATH", &path)
        .status()?;
    if !status.success() {
        panic!("npm install failed");
    }

    let status = Command::new(npm_exe)
        .arg("run")
        .arg("build")
        .env("PATH", &path)
        .current_dir(frontend_dir)
        .status()?;
    if !status.success() {
        panic!("npm run build failed");
    }


    println!("Generating static...");
    // 2. Пути к итоговым файлам
    let build_dir = frontend_dir.join("public/build");
    let index_html_path = frontend_dir.join("public/index.html");
    let js_path = build_dir.join("bundle.js");
    let css_path = build_dir.join("bundle.css");

    // 3. Читаем файлы
    let index_html = fs::read_to_string(index_html_path)?;
    let js = fs::read_to_string(js_path)?;
    let css = fs::read_to_string(css_path)?;

    // 4. Генерируем Rust модуль
    let out_dir = std::env::var("OUT_DIR")?;
    println!("{out_dir}");
    let dest_path = Path::new(&out_dir).join("static_files.rs");
    let content = format!(
        "pub const INDEX_HTML: &str = r#\"{}\"#;\n\
         pub const JS: &str = r#\"{}\"#;\n\
         pub const CSS: &str = r#\"{}\"#;\n",
        index_html, js, css
    );
    fs::write(dest_path, content)?;

    println!("cargo:rerun-if-changed=frontend/src");
    println!("cargo:rerun-if-changed=frontend/public/index.html");

    Ok(())
}
