use std::ffi::OsStr;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub(crate) enum FileType {
    Properties,
    Hocon,
    Json,
    All,
}

#[derive(Default, Debug)]
pub(crate) struct FileRead {
    pub(crate) properties: Option<String>,
    pub(crate) json: Option<String>,
    pub(crate) hocon: Option<String>,
}
impl FileRead {
    fn from_file_type(ft: &FileType, s: String) -> Self {
        match ft {
            FileType::Properties => Self {
                properties: Some(s),
                ..Default::default()
            },
            FileType::Json => Self {
                json: Some(s),
                ..Default::default()
            },
            FileType::Hocon => Self {
                hocon: Some(s),
                ..Default::default()
            },
            FileType::All => unimplemented!(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ConfFileMeta {
    path: PathBuf,
    file_name: String,
    full_path: PathBuf,
    file_type: FileType,
}
impl ConfFileMeta {
    pub(crate) fn from_path(path: PathBuf) -> Self {
        let file = path
            .file_name()
            .expect("got a path without a filename")
            .to_str()
            .expect("got invalid UTF-8 path");
        let mut parent_path = path.clone();
        parent_path.pop();

        Self {
            path: parent_path,
            file_name: String::from(file),
            full_path: path.clone(),
            file_type: match Path::new(file).extension().and_then(OsStr::to_str) {
                Some("properties") => FileType::Properties,
                Some("json") => FileType::Json,
                Some("conf") => FileType::Hocon,
                _ => FileType::All,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct HoconLoaderConfig {
    pub(crate) include_depth: u8,
    pub(crate) file_meta: Option<ConfFileMeta>,
    pub(crate) system: bool,
    #[cfg(feature = "url-support")]
    pub(crate) external_url: bool,
    pub(crate) strict: bool,
    pub(crate) max_include_depth: u8,
}

impl Default for HoconLoaderConfig {
    fn default() -> Self {
        Self {
            include_depth: 0,
            file_meta: None,
            system: true,
            #[cfg(feature = "url-support")]
            external_url: true,
            strict: false,
            max_include_depth: 10,
        }
    }
}

impl HoconLoaderConfig {
    pub(crate) fn included_from(&self) -> Self {
        Self {
            include_depth: self.include_depth + 1,
            ..self.clone()
        }
    }

    pub(crate) fn with_file(&self, path: PathBuf) -> Self {
        match self.file_meta.as_ref() {
            Some(file_meta) => Self {
                file_meta: Some(ConfFileMeta::from_path(file_meta.clone().path.join(path))),
                ..self.clone()
            },
            None => Self {
                file_meta: Some(ConfFileMeta::from_path(path)),
                ..self.clone()
            },
        }
    }

    pub(crate) fn parse_str_to_internal(
        &self,
        s: FileRead,
    ) -> Result<crate::internals::HoconInternal, crate::Error> {
        let mut internal = crate::internals::HoconInternal::empty();
        if let Some(properties) = s.properties {
            internal = internal.add(
                java_properties::read(properties.as_bytes())
                    .map(crate::internals::HoconInternal::from_properties)
                    .map_err(|_| crate::Error::Parse)?,
            );
        };
        if let Some(json) = s.json {
            internal = internal.add(
                crate::parser::root(format!("{}\n\0", json).as_bytes(), self)
                    .map_err(|_| crate::Error::Parse)
                    .and_then(|(remaining, parsed)| {
                        if Self::remaining_only_whitespace(remaining) {
                            parsed
                        } else if self.strict {
                            Err(crate::Error::Deserialization {
                                message: String::from("file could not be parsed completely"),
                            })
                        } else {
                            parsed
                        }
                    })?,
            );
        };
        if let Some(hocon) = s.hocon {
            internal = internal.add(
                crate::parser::root(format!("{}\n\0", hocon).as_bytes(), self)
                    .map_err(|_| crate::Error::Parse)
                    .and_then(|(remaining, parsed)| {
                        if Self::remaining_only_whitespace(remaining) {
                            parsed
                        } else if self.strict {
                            Err(crate::Error::Deserialization {
                                message: String::from("file could not be parsed completely"),
                            })
                        } else {
                            parsed
                        }
                    })?,
            );
        };

        Ok(internal)
    }

    fn remaining_only_whitespace(remaining: &[u8]) -> bool {
        remaining
            .iter()
            .find(|c| **c != 10 && **c != 0)
            .map(|_| false)
            .unwrap_or(true)
    }

    pub(crate) fn read_file_to_string(path: PathBuf) -> Result<String, failure::Error> {
        let mut file = File::open(path.as_os_str())?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        Ok(contents)
    }

    pub(crate) fn read_file(&self) -> Result<FileRead, failure::Error> {
        let full_path = self
            .file_meta
            .clone()
            .expect("missing file metadata")
            .full_path;
        match self.file_meta.as_ref().map(|fm| &fm.file_type) {
            Some(FileType::All) => Ok(FileRead {
                hocon: Self::read_file_to_string({
                    let mut path = full_path.clone();
                    path.set_extension("conf");
                    path
                })
                .ok(),
                json: Self::read_file_to_string({
                    let mut path = full_path.clone();
                    path.set_extension("json");
                    path
                })
                .ok(),
                properties: Self::read_file_to_string({
                    let mut path = full_path;
                    path.set_extension("properties");
                    path
                })
                .ok(),
            }),
            Some(ft) => Ok(FileRead::from_file_type(
                ft,
                Self::read_file_to_string(full_path)?,
            )),
            _ => unimplemented!(),
        }
    }

    #[cfg(feature = "url-support")]
    pub(crate) fn load_url(
        &self,
        url: &str,
    ) -> Result<crate::internals::HoconInternal, failure::Error> {
        if let Ok(parsed_url) = reqwest::Url::parse(url) {
            if parsed_url.scheme() == "file" {
                if let Ok(path) = parsed_url.to_file_path() {
                    let include_config = self.included_from().with_file(path);
                    let s = include_config.read_file()?;
                    Ok(include_config.parse_str_to_internal(s).map_err(|_| {
                        crate::Error::Include {
                            path: String::from(url),
                        }
                    })?)
                } else {
                    Err(crate::Error::Include {
                        path: String::from(url),
                    }
                    .into())
                }
            } else if self.external_url {
                let body = reqwest::get(parsed_url)
                    .and_then(|mut r| r.text())
                    .map_err(|_| crate::Error::Include {
                        path: String::from(url),
                    })?;

                Ok(self.parse_str_to_internal(FileRead {
                    hocon: Some(body),
                    ..Default::default()
                })?)
            } else {
                Err(crate::Error::Include {
                    path: String::from(url),
                }
                .into())
            }
        } else {
            Err(crate::Error::Include {
                path: String::from(url),
            }
            .into())
        }
    }
}
