#!/usr/bin/env run-cargo-script
//! This is a regular crate doc comment, but it also contains a partial
//! Cargo manifest.  Note the use of a *fenced* code block, and the
//! `cargo` "language".
//!
//! ```cargo
//! [dependencies]
//! ```

fn main() {
    let dir_entries = std::fs::read_dir("posts").unwrap();
    for entry in dir_entries {
        let entry = entry.unwrap();
        let path = entry.path();
        let file_name = format!("{}/index.md", path.display());
        let mut content = std::fs::read_to_string(&file_name);
        if let Ok(content) = content {
            let metadata_section: Vec<&str> = content
                .lines()
                .skip(1)
                .take_while(|line| !line.starts_with("+++"))
                .collect::<Vec<_>>();
            let mut new_metadata = metadata_section
                .iter()
                .map(|line| line.replace(" =", ":").replace("=", ":").trim().to_string())
                .filter(|line| !line.contains("\"\""))
                .filter(|line| !(line.contains("showFullContent") || line.contains("keywords")))
                .collect::<Vec<_>>();
            let name = new_metadata
                .iter()
                .find(|line| line.starts_with("title:"))
                .unwrap()
                .split(":")
                .last()
                .unwrap()
                .trim()
                .replace("\"", "");
            let url = format!("url: /posts/{}", entry.file_name().to_str().unwrap());
            new_metadata.push(url);
            println!("New metadata: {:?}", new_metadata);
            let text = content
                .lines()
                .skip_while(|line| !line.starts_with("+++"))
                .skip(1)
                .skip_while(|line| !line.starts_with("+++"))
                .skip(1)
                .collect::<Vec<_>>()
                .join("\n")
                .replace("./images/", "/images/")
                .replace(".jpg", ".avif")
                .replace(".jpeg", ".avif")
                .replace(".png", ".avif");
            let output = format!("---\n{}\n---\n\n{}", new_metadata.join("\n"), text);
            let new_file_name = format!("posts/{}.md", name);
            std::fs::write(&new_file_name, output).unwrap();
        }
    }
}
