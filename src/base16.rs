#![allow(non_snake_case)]
use crossterm::style::Color;
use reqwest::blocking::{Client, Response};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::StatusCode;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use url::Url;

use serde::Deserialize;
use zip::ZipArchive;

use crate::errors::{CliError, CliResult};
use crate::style::{parse_hex_color, Theme};

const THEME_LIST_URL: &str = "https://raw.githubusercontent.com/chriskempson/base16-schemes-source/master/list.yaml";

#[derive(Deserialize)]
pub struct Base16 {
  pub scheme: String,
  pub author: String,
  pub base00: String,
  pub base01: String,
  pub base02: String,
  pub base03: String,
  pub base04: String,
  pub base05: String,
  pub base06: String,
  pub base07: String,
  pub base08: String,
  pub base09: String,
  pub base0A: String,
  pub base0B: String,
  pub base0C: String,
  pub base0D: String,
  pub base0E: String,
  pub base0F: String,
}

#[derive(Deserialize)]
struct Repo {
  pub default_branch: String,
}

pub fn get_themes_list() -> CliResult<HashMap<String, String>> {
  let mut resp = String::new();
  let _ = reqwest::blocking::get(THEME_LIST_URL)
    .map_err(|source| CliError::wrap(Box::new(source)))?
    .read_to_string(&mut resp)
    .map_err(|source| CliError::wrap(Box::new(source)))?;

  serde_yaml::from_str(&resp)
    .map_err(|source| CliError::wrap(Box::new(source)))
}

pub fn get_theme_zip(url: &str) -> CliResult<File> {
  // https://github.com/casonadams/base16-apprentice-scheme/archive/refs/heads/main.zip
  let repo_url =
    Url::parse(url).map_err(|source| CliError::Parse {
      message: source.to_string(),
    })?;

  let mut headers = HeaderMap::new();
  let ua = HeaderValue::from_str("yatt").unwrap();
  headers.insert("User-Agent", ua);
  let cli =
    Client::builder().default_headers(headers).build().unwrap();

  let zip_url = repo_url.clone();

  let mut resp = None;

  for branch in ["master", "main"] {
    let r = query_file(&cli, &zip_url, branch)?;

    if r.status() == StatusCode::OK {
      resp = Some(r);
      break;
    }
  }

  if resp.is_none() {
    let branch = get_default_branch(&cli, repo_url.path())?;
    let r = query_file(&cli, &zip_url, branch.as_str())?;

    if r.status() == StatusCode::OK {
      resp = Some(r);
    }
  }

  if resp.is_none() {
    return Err(CliError::Cmd {
      message: "download failed".into(),
    });
  }
  let mut resp = resp.unwrap();
  let mut buffer = [0u8; 4096];
  let mut zip_file = tempfile::tempfile().unwrap();

  loop {
    let bts = resp.read(&mut buffer).unwrap();
    let _ = zip_file.write(&buffer[..bts]).unwrap();
    if bts == 0 {
      break;
    }
  }

  Ok(zip_file)
}

fn get_default_branch(
  cli: &Client,
  repo_path: &str,
) -> CliResult<String> {
  let mut repo_api_url =
    Url::parse("https://api.github.com").unwrap();
  repo_api_url.set_path(format!("repos{}", repo_path).as_str());
  let repo = cli
    .get(repo_api_url)
    .send()
    .map_err(|source| CliError::wrap(Box::new(source)))?
    .json::<Repo>()
    .map_err(|source| CliError::Parse {
      message: source.to_string(),
    })?;

  Ok(repo.default_branch)
}

fn query_file(
  cli: &Client,
  file_url: &Url,
  branch: &str,
) -> CliResult<Response> {
  let mut file_url = file_url.clone();
  file_url.set_path(
    format!("{}/archive/refs/heads/{}.zip", &file_url.path(), branch)
      .as_str(),
  );

  cli
    .get(file_url.as_str())
    .send()
    .map_err(|source| CliError::wrap(Box::new(source)))
}

pub fn get_themes_from_zip(
  file: File,
) -> CliResult<HashMap<String, CliResult<Base16>>> {
  let mut zip_file = ZipArchive::new(file)
    .map_err(|source| CliError::wrap(Box::new(source)))?;
  let names: Vec<String> = zip_file
    .file_names()
    .filter(|name| {
      let name = name.to_lowercase();
      if name.ends_with(".yaml") || name.ends_with(".yml") {
        return true;
      }
      false
    })
    .map(String::from)
    .collect();

  let mut res: HashMap<String, CliResult<Base16>> = HashMap::new();

  for name in names {
    let theme: CliResult<Base16> =
      serde_yaml::from_reader(zip_file.by_name(&name).unwrap())
        .map_err(|source| {
          if source.to_string().contains("missing field") {
            CliError::Parse {
              message: source.to_string(),
            }
          } else {
            CliError::wrap(Box::new(source))
          }
        });

    if let Err(CliError::Parse { .. }) = theme {
      continue;
    }
    res.insert(
      Path::new(&name)
        .with_extension("")
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .into(),
      theme,
    );
  }

  Ok(res)
}

pub fn convert_themes(
  themes: &HashMap<String, Base16>,
) -> HashMap<String, Theme> {
  let mut res: HashMap<String, Theme> = HashMap::new();
  let default_theme = Theme::default();

  for (k, v) in themes {
    let c1 = parse_hex_color(format!("#{}", v.base0D).as_str())
      .unwrap_or(default_theme.c1);
    let c2 = parse_hex_color(format!("#{}", v.base0B).as_str())
      .unwrap_or(default_theme.c2);
    let c3 = parse_hex_color(format!("#{}", v.base0A).as_str())
      .unwrap_or(default_theme.c3);
    let c4 = parse_hex_color(format!("#{}", v.base08).as_str())
      .unwrap_or(default_theme.c4);
    let c5 = parse_hex_color(format!("#{}", v.base05).as_str())
      .unwrap_or(Color::White);

    res.insert(k.to_string(), Theme { c1, c2, c3, c4, c5 });
  }

  res
}
