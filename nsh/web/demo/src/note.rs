use std::fmt;

/// One note: its text and the tags written in it as `#tag`.
#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    pub text: String,
    pub tags: Vec<String>,
}

impl Note {
    /// Reads a line such as `Buy milk #home #today`.
    pub fn parse(line: &str) -> Note {
        let mut text = Vec::new();
        let mut tags = Vec::new();
        for word in line.split_whitespace() {
            match word.strip_prefix('#') {
                Some(tag) if !tag.is_empty() => tags.push(tag.to_lowercase()),
                _ => text.push(word),
            }
        }
        Note { text: text.join(" "), tags }
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }
}

impl fmt::Display for Note {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.text)?;
        for tag in &self.tags {
            write!(f, " #{tag}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_come_out_of_the_text() {
        let n = Note::parse("Buy milk #Home");
        assert_eq!(n.text, "Buy milk");
        assert_eq!(n.tags, ["home"]);
    }
}
