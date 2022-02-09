pub use anyhow::{Context,Result};
pub use termion::{cursor, clear, style, input::TermRead, raw::IntoRawMode};
pub use termion::event::{Event, Key};
pub use libsufec::{Account as SufecAccount, Message, MessageContent, SufecAddr};
pub use serde::{Deserialize, Serialize};
pub use sodiumoxide::crypto::box_::{self, PublicKey, SecretKey};
pub use sodiumoxide::randombytes::randombytes;
pub use std::io::{stdin, stdout, Write};
pub use std::fs::File;
pub use std::cmp::{min, max};
pub use std::collections::HashMap;
pub use std::sync::{mpsc, Arc, RwLock};
pub use std::time::UNIX_EPOCH;

#[derive(Serialize, Deserialize, Clone)]
pub struct Contact {
	pub addr: SufecAddr,
	pub name: String,
}
#[derive(Serialize, Deserialize, Clone)]
pub struct Account {
	pub account: SufecAccount,
	pub contacts: Vec<Contact>,
	pub eph_pub: PublicKey,
	pub eph_sec: SecretKey,
	pub rooms: Vec<Room>,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Room {
	pub id: RoomId,
	pub name: String,
	pub members: Vec<SufecAddr>,
	pub history: Vec<HistoryEntry>,
	pub unseen: u16,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HistoryEntry {
	pub sender: SufecAddr,
	pub timestamp: u64,
	pub msg: MessageContent,
}

pub type RoomId = [u8; 2];
// The Mode determines what's showing on the left pane.
#[derive(PartialEq, Eq)]
pub enum Mode {
	Rooms,
	Members,
	Contacts,
}

pub struct State {
	pub room_id: RoomId,
	pub scroll: HashMap<RoomId, i16>,
	pub msg_buf: String,
	pub width: u16,
	pub height: u16,
	pub mode: Mode,
	pub selected_index: usize,
}
impl State {
	pub fn new(account: &Account) -> Self {
		let (width, height) = termion::terminal_size().unwrap();
		let mut scroll = HashMap::new();
		for room in &account.rooms {
			scroll.insert(room.id, 0);
		}
		Self {
			room_id: account.rooms.get(0).map(|r| r.id).unwrap_or_default(),
			msg_buf: String::new(),
			width, height, scroll, selected_index: 0,
			mode: Mode::Rooms,
		}
	}
}

pub fn load_account() -> Result<Account> {
	let f = File::open("account.ron").context("couldn't open account.ron")?;
	ron::de::from_reader(f).context("couldn't parse account.ron")
}
pub fn save_account(account: &Account) -> Result<()> {
	write_ron(account, "account.ron").context("couldn't save account")

}
pub fn write_ron<T: Serialize>(t: &T, path: &str) -> Result<()> {
	let f = File::create(path).context("couldn't create file")?;
	ron::ser::to_writer_pretty(f, t, ron::ser::PrettyConfig::default())?;
	Ok(())
}
