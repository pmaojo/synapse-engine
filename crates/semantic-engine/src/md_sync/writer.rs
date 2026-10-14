use super::parser::{strip_backlinks, BACKLINKS_HEADING};
use std::fs;
use std::path::Path;

pub struct MarkdownWriter;

impl MarkdownWriter {
    /// Rewrites the backlinks section at the end of a Markdown file with the
    /// given `(predicate, subject)` pairs. Removes the section when there are
    /// none, and leaves the file untouched when nothing changes.
    pub fn write_backlinks<P: AsRef<Path>>(path: P, backlinks: &[(String, String)]) -> Result<(), anyhow::Error> {
        let content = fs::read_to_string(&path)?;
        let body = strip_backlinks(&content).trim_end();

        let new_content = if backlinks.is_empty() {
            format!("{body}\n")
        } else {
            let mut text = format!("{body}\n\n{BACKLINKS_HEADING}\n");
            for (pred, subject) in backlinks {
                text.push_str(&format!("- **{pred}**: [[{subject}]]\n"));
            }
            text
        };

        if new_content != content {
            fs::write(path, new_content)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_replaces_and_removes_the_section() {
        let dir = std::env::temp_dir().join(format!("synapse-writer-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("a.md");
        fs::write(&file, "# A\n\nBody\n").unwrap();

        MarkdownWriter::write_backlinks(&file, &[("mentions".into(), "b".into())]).unwrap();
        let once = fs::read_to_string(&file).unwrap();
        assert!(once.ends_with("## 🧠 Synapse Backlinks\n- **mentions**: [[b]]\n"));

        MarkdownWriter::write_backlinks(&file, &[("mentions".into(), "b".into())]).unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), once);

        MarkdownWriter::write_backlinks(&file, &[]).unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), "# A\n\nBody\n");
        fs::remove_dir_all(dir).unwrap();
    }
}
