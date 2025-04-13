use chrono::NaiveDate;
use pulldown_cmark::{html, Options, Parser};
use regex::Regex;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct FrontMatter {
    pub title: String,
    pub category: String,
    pub id: String,
    pub tags: Vec<String>,
    pub created_time: NaiveDate,
    pub last_edited_time: NaiveDate,
    pub status: String,
}

#[derive(Debug)]
pub struct Article {
    pub fm: FrontMatter,
    pub html: String,
    pub slug: String,
}

fn extract_frontmatter(content: &str) -> Option<FrontMatter> {
    let re = Regex::new(r"(?s)\A\+\+\+\n(.*?)\n\+\+\+").unwrap();
    let caps = re.captures(content)?;
    let frontmatter_str = caps.get(1)?.as_str();

    toml::from_str(frontmatter_str).ok()
}

pub fn parse(src: &str, slug: &str) -> anyhow::Result<Article> {
    let result = Matter::<YAML>::new().parse(src);
    let fm: FrontMatter = result
        .data
        .ok_or_else(|| anyhow::anyhow!("no frontmatter"))?
        .deserialize()?;

    let mut html_out = String::new();
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    html::push_html(&mut html_out, Parser::new_ext(&result.content, opts));

    Ok(Article {
        fm,
        html: html_out,
        slug: slug.to_string(),
    })
}
