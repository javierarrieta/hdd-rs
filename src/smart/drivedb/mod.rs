mod parser;
mod presets;
pub use self::parser::Entry;

use std::fs::File;
use std::io::prelude::*;
use std::io;

use nom;

use std::{error, fmt, convert};

use super::data::id;

use regex::bytes::Regex;

use std::collections::HashMap;

#[derive(Debug)]
pub enum Error {
	IO(io::Error),
	Parse, // TODO? Parse(nom::verbose_errors::Err) if dependencies.nom.features = ["verbose-errors"]
}
impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			Error::IO(ref err) => write!(f, "IO error: {}", err),
			Error::Parse => write!(f, "Parse error"),
		}
	}
}
impl error::Error for Error {
	fn description(&self) -> &str {
		match *self {
			Error::IO(ref err) => err.description(),
			Error::Parse => "malformed database",
		}
	}
	fn cause(&self) -> Option<&error::Error> {
		match *self {
			Error::IO(ref err) => Some(err),
			Error::Parse => None,
		}
	}
}
impl convert::From<io::Error> for Error {
	fn from(err: io::Error) -> Error { Error::IO(err) }
}

pub fn load(file: &str) -> Result<Vec<Entry>, Error> {
	let mut db = Vec::new();
	File::open(&file)?.read_to_end(&mut db)?;

	match parser::database(&db) {
		nom::IResult::Done(_, entries) => Ok(entries),
		nom::IResult::Error(_) => Err(Error::Parse),
		nom::IResult::Incomplete(_) => unreachable!(), // XXX is it true?
	}
}

fn merge_presets(default: &Option<HashMap<u8, String>>, drive: &Option<HashMap<u8, String>>) -> HashMap<u8, String> {
	let mut output = HashMap::<u8, String>::new();
	if let Some(ref dpresets) = *default {
		for (id, name) in dpresets {
			output.insert(*id, name.clone());
		}
	}
	if let Some(ref dpresets) = *drive {
		for (id, name) in dpresets {
			output.insert(*id, name.clone());
		}
	}
	output
}

#[derive(Debug)]
pub enum Match<'a> {
	Default { presets: HashMap<u8, String> },
	Found {
		family: &'a String,
		warning: &'a String, // TODO Option<>
		presets: HashMap<u8, String>,
	}
}

pub fn match_entry<'a>(id: &id::Id, db: &'a Vec<Entry>) -> Match<'a> {
	let mut db = db.iter();
	let _ = db.next(); // skip dummy svn-id entry
	let default = db.next().unwrap(); // I'm fine with panicking in the absence of default entry (XXX)

	for entry in db {
		// TODO? put compiled `regex::Regex`es right in the `struct Entry`. This would be beneficial for lib users that test drives in bulk, less so for one-time users with popular drives
		// TODO invalid regex should result in parsing error (or maybe not, maybe just stick to Option<Regex>)

		// USB ID entries are parsed differently; also, we don't support USB devices yet
		if entry.model.starts_with("USB:") { continue }

		// model and firmware are expected to be ascii strings, no need to try matching unicode characters

		// > [modelregexp] should never be "".
		let re = Regex::new(format!("(?-u)^{}$", entry.model).as_str()).unwrap();
		if !re.is_match(id.model.as_bytes()) { continue }

		if entry.firmware.len() > 0 {
			let re = Regex::new(format!("^(?-u){}$", entry.firmware).as_str()).unwrap();
			if !re.is_match(id.firmware.as_bytes()) { continue }
		}

		// > The table will be searched from the start to end or until the first match
		return Match::Found {
			family: &entry.family,
			warning: &entry.warning,
			presets: merge_presets(
				&presets::parse(&default.presets),
				&presets::parse(&entry.presets),
			),
		};
	}

	Match::Default {
		presets: presets::parse(&default.presets).unwrap(), // again, panic is kinda ok here, we're expecting default entry to have all that (XXX)
	}
}