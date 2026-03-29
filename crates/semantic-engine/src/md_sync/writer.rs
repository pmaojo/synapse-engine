use regex::Regex;
use std::fs;
use std::path::Path;

pub struct MarkdownWriter;

impl MarkdownWriter {
    /// Inyecta las inferencias calculadas (Backlinks, relaciones derivadas, etc.) al final del archivo MD.
    pub fn write_inferences<P: AsRef<Path>>(
        path: P,
        inferences: &[(String, String)],
    ) -> Result<(), anyhow::Error> {
        let content = fs::read_to_string(&path)?;

        let header = "## 🧠 Synapse Backlinks\n";
        let mut text = String::new();
        text.push_str(header);

        for (pred, obj) in inferences {
            text.push_str(&format!("- **{}**: [[{}]]\n", pred, obj));
        }

        // Si ya existe la sección, reemplazarla.
        let re = Regex::new(r"## 🧠 Synapse Backlinks[\s\S]*").unwrap();
        let new_content = if re.is_match(&content) {
            re.replace(&content, text.as_str()).to_string()
        } else {
            format!("{}\n{}", content.trim_end(), text)
        };

        fs::write(path, new_content)?;
        Ok(())
    }

    /// Actualiza silenciosamente el YAML Frontmatter con nuevos metadatos directos de la API.
    pub fn update_frontmatter<P: AsRef<Path>>(
        path: P,
        new_props: std::collections::HashMap<String, String>,
    ) -> Result<(), anyhow::Error> {
        let content = fs::read_to_string(&path)?;

        // Expresión para buscar bloque YAML Frontmatter
        let re = Regex::new(r"(?s)^---\n(.*?)\n---").unwrap();

        let new_content = if let Some(caps) = re.captures(&content) {
            let mut yaml_text = caps[1].to_string();
            // Muy rudimentario: añadimos o reemplazamos las props al final del YAML
            for (k, v) in new_props {
                let re_prop = Regex::new(&format!(r"(?m)^{}:.*$", k)).unwrap();
                let prop_line = format!("{}: {}", k, v);
                if re_prop.is_match(&yaml_text) {
                    yaml_text = re_prop.replace(&yaml_text, prop_line.as_str()).to_string();
                } else {
                    yaml_text.push_str(&format!("\n{}", prop_line));
                }
            }
            re.replace(&content, format!("---\n{}\n---", yaml_text).as_str()).to_string()
        } else {
            // No hay Frontmatter, crearlo
            let mut yaml_text = String::new();
            yaml_text.push_str("---\n");
            for (k, v) in new_props {
                yaml_text.push_str(&format!("{}: {}\n", k, v));
            }
            yaml_text.push_str("---\n\n");
            format!("{}{}", yaml_text, content)
        };

        fs::write(path, new_content)?;
        Ok(())
    }
}
