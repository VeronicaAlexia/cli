use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, ensure, Result};
use pulldown_cmark::{Event, Options, Parser};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{cmd::Convert, utils};

#[must_use]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct MetaData {
    pub title: String,
    pub author: String,
    pub lang: String,
    pub description: Option<String>,
    pub cover_image: Option<PathBuf>,
}

impl MetaData {
    pub(crate) fn lang_is_ok(&self) -> bool {
        self.lang == "zh-Hant" || self.lang == "zh-Hans"
    }

    pub(crate) fn cover_image_is_ok(&self) -> bool {
        !novel_api::is_some_and(self.cover_image.as_ref(), |path| !path.is_file())
    }
}

pub(crate) fn read_markdown<T>(markdown_path: T) -> Result<(MetaData, String)>
where
    T: AsRef<Path>,
{
    let markdown_path = markdown_path.as_ref();
    ensure!(
        utils::is_markdown(markdown_path),
        "The input file is not in markdown format"
    );

    let bytes = fs::read(markdown_path)?;
    let markdown = simdutf8::basic::from_utf8(&bytes)?;

    ensure!(
        markdown.starts_with("---"),
        "The markdown format is incorrect, it should start with `---`"
    );

    if let Some(index) = markdown.find("\n...\n") {
        let yaml = &markdown[4..index];

        let meta_data: MetaData = serde_yaml::from_str(yaml)?;
        let markdown = markdown[index + 5..].to_string();

        Ok((meta_data, markdown))
    } else {
        bail!("The markdown format is incorrect, it should end with `...`");
    }
}

pub(crate) fn to_events<T>(markdown: &str, converts: T) -> Result<Vec<Event>>
where
    T: AsRef<[Convert]> + Sync,
{
    let mut options = Options::all();
    options.remove(Options::ENABLE_SMART_PUNCTUATION);
    let parser = Parser::new_ext(markdown, options);

    let events = parser.collect::<Vec<_>>();
    let iter = events.into_par_iter().map(|event| match event {
        Event::Text(text) => {
            Event::Text(utils::convert_str(text, converts.as_ref()).unwrap().into())
        }
        _ => event.to_owned(),
    });

    Ok(iter.collect::<Vec<Event>>())
}
