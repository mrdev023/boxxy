use regex::Regex;

pub struct BookmarkTemplate {
    pub raw_script: String,
    pub placeholders: Vec<String>,
}

impl BookmarkTemplate {
    pub fn parse(script: &str) -> Self {
        // Regex to match {{{name}}}
        let re = Regex::new(r"\{\{\{([^}]+)\}\}\}").unwrap();
        let mut placeholders = Vec::new();

        for cap in re.captures_iter(script) {
            let name = cap[1].trim().to_string();
            if !placeholders.contains(&name) {
                placeholders.push(name);
            }
        }

        Self {
            raw_script: script.to_string(),
            placeholders,
        }
    }

    pub fn render(&self, values_str: &str) -> String {
        let values: Vec<String> = values_str
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();

        let mut rendered = self.raw_script.clone();

        for (i, name) in self.placeholders.iter().enumerate() {
            if let Some(val) = values.get(i) {
                // If value is empty, don't replace or replace with empty
                // We'll replace with actual value provided
                let pattern = format!("{{{{{{{}}}}}}}", name);
                rendered = rendered.replace(&pattern, val);
            }
        }

        rendered
    }

    pub fn get_default_input(&self) -> String {
        self.placeholders.join(", ")
    }

    pub fn has_placeholders(&self) -> bool {
        !self.placeholders.is_empty()
    }
}
