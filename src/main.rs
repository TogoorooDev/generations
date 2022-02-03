use anyhow::{Context,Result};
use termion::{cursor, clear, input::TermRead, raw::IntoRawMode};
use termion::event::Key;
use libsufec::{Account as SufecAccount, Message, MessageContent, SufecAddr};
use serde::{Deserialize, Serialize};
use sodiumoxide::crypto::box_::{self, PublicKey, SecretKey};
use std::io::{stdin, stdout, Write};
use std::fs::File;
use std::sync::{mpsc, Arc, RwLock};
use std::time::UNIX_EPOCH;

#[derive(Serialize, Deserialize, Clone)]
pub struct Contact {
	addr: SufecAddr,
	name: String,
}
#[derive(Serialize, Deserialize, Clone)]
pub struct Account {
	account: SufecAccount,
	contacts: Vec<Contact>,
	eph_pub: PublicKey,
	eph_sec: SecretKey,
	rooms: Vec<Room>,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
struct Room {
	id: [u8; 2],
	name: String,
	members: Vec<SufecAddr>,
	history: Vec<HistoryEntry>,
	unseen: u16,
}
impl PartialEq for Room {
	fn eq(&self, other: &Self) -> bool {
		self.members == other.members
	}
}
impl Eq for Room {}
#[derive(Serialize, Deserialize, Clone, Debug)]
struct HistoryEntry {
	sender: SufecAddr,
	timestamp: u64,
	msg: MessageContent,
}

struct State {
	room_id: [u8; 2],
	msg_buf: String,
	width: u16,
	height: u16,
}

fn clear() { println!("{}{}", clear::All, cursor::Goto(1, 1)); }

fn goto(x: u16, y: u16) { println!("{}", cursor::Goto(x, y)); }

fn main() -> Result<()> {
	sodiumoxide::init().expect("sodiumoxie::init failed");
	let account = Arc::new(RwLock::new(load_account()?));
	let (width, height) = termion::terminal_size().unwrap();
	let state = Arc::new(RwLock::new(State{
			room_id: account.read().unwrap().rooms[0].id,
			msg_buf: String::new(),
			width, height,
	}));
	let accountclone = Arc::clone(&account);
	let stateclone = Arc::clone(&state);
	let receive_msg = move |from, timestamp, msg| {
		let mut account = accountclone.write().unwrap();
		let state = stateclone.read().unwrap();
		message_callback(&mut account, &state, from, timestamp, msg);
		write_ron(&account.clone(), "account.ron").unwrap();
	};
	let accountclone = Arc::clone(&account);
	std::thread::spawn(|| sufec_backend(accountclone, receive_msg));

	let mut _stdout = stdout().into_raw_mode().unwrap();
	prep(width, height, &account.read().unwrap().rooms);
	
	loop {
		let (nwidth, nheight) = termion::terminal_size().unwrap();
		if (nwidth, nheight) != (width, height){
			let mut state = state.write().unwrap();
			state.width = nwidth;
			state.height = nheight;
			prep(nwidth, nheight, &account.read().unwrap().rooms);
		}

		for event in stdin().keys() {
			match event.unwrap() {
			Key::Esc => {
				quit_menu();
			},
			Key::Char(c) => {
				let mut state = state.write().unwrap();
				if c == '\n' {
					print!("{}{}", clear::CurrentLine, cursor::Goto(1, height));
					stdout().flush().unwrap();
					send_message(&mut account.write().unwrap(), &mut state);
				} else {
					print!("{}", c);
					state.msg_buf.push(c);
				}
				stdout().flush().unwrap();
			},
			_ => {},
			}
		}
	}
}

fn prep(width: u16, height: u16, rooms: &[Room]){
	let sep: u16 = width / 3;
	clear();
	draw_rooms(height, sep, rooms);
	draw_bottom(height, width);
	print!("{}", cursor::Goto(1, height));
	stdout().flush().unwrap();
}

fn quit_menu(){
	clear();
	std::process::exit(0);
}

fn draw_rooms(height: u16, sep: u16, rooms: &[Room]) {
	// draw separator bar
	for y in 1..height-1 {
		print!("{}|", cursor::Goto(sep, y));
	}
	let mut y = 1;
	for room in rooms {
		// draw room name
		print!("{}|{}", cursor::Goto(1, y), room.name);
		// go down and draw a separator line before the next room
		y += 1;
		print!("{}|{}", cursor::Goto(1, y), "-".repeat((sep - 2) as usize));
		y += 1;
	}
}

fn draw_bottom(height: u16, width: u16) {
	print!("{}{}", cursor::Goto(1, height-1), "-".repeat(width as usize));
}

fn draw_messages(state: &State, messages: &[HistoryEntry]) {
	let sep = state.width / 3;
	let message_width = state.width - sep;
	// we don't have automatic GUI-like scrolling, therefore we start from the end
	let mut y = state.height - 2;
	for message in messages.iter().rev() {
		let text = match &message.msg {
			MessageContent::Text(s) => s,
			_ => unimplemented!(),
		};
		let chars: Vec<char> = text.chars().collect();
		let lines = chars.len() as u16 / message_width + 1;
		// Go to the beginning of where the message will span.
		y -= lines - 1;
		goto(sep+1, y);
		let mut index: usize = 0;
		for i in 0..lines {
			let end = std::cmp::min(index+message_width as usize, chars.len());
			let slice = chars[index..end].iter().collect::<String>();
			print!("{}{}{}", cursor::Goto(sep+1, y+i), clear::UntilNewline, slice);
			index += message_width as usize;
		}
		y -= 1;
	}
	// Reset the cursor position.
	print!("{}", cursor::Goto(1+state.msg_buf.len() as u16, state.height));
	stdout().flush().unwrap();
}

fn load_account() -> Result<Account> {
	let f = File::open("account.ron").context("couldn't open account.ron")?;
	ron::de::from_reader(f).context("couldn't parse account.ron")
}

fn write_ron<T: Serialize>(t: &T, path: &str) -> Result<()> {
	let f = File::create(path).context("couldn't create file")?;
	ron::ser::to_writer_pretty(f, t, ron::ser::PrettyConfig::default())?;
	Ok(())
}

fn send_message(account: &mut Account, state: &mut State) {
	let content = MessageContent::Text(state.msg_buf.clone());
	state.msg_buf.clear();
	// Find the room.
	let room = account.rooms.iter_mut().find(|r| r.id == state.room_id).unwrap();
	// Add it to the history.
	let timestamp = UNIX_EPOCH.elapsed().unwrap().as_micros() as u64;
	let history_entry = HistoryEntry{
		sender: account.account.addr.clone(),
		timestamp,
		msg: content.clone(),
	};
	room.history.push(history_entry);
	draw_messages(state, &room.history);
	for recipient in room.members.iter() {
		let other_recipients = room.members.iter().filter(|r| *r != recipient).map(|r| r.clone()).collect();
		let message = Message{other_recipients, content: content.clone()};
		if let Err(e) = libsufec::send(&account.account, recipient, message) {
			eprintln!("couldn't send to {:?}: {}", recipient, e);
		}
	}
}

fn message_callback(account: &mut Account, state: &State, from: SufecAddr, timestamp: u64, msg: Message) {
	// Build a sorted list of users to match to one of our rooms.
	let mut recipients = msg.other_recipients.clone();
	recipients.push(from.clone());
	recipients.sort_unstable_by_key(|addr| addr.id.0);
	// See if we have a room that matches.
	let room = account.rooms.iter_mut().find(|r| {
		let mut room_members = r.members.clone();
		room_members.sort_unstable_by_key(|addr| addr.id.0);
		recipients == room_members
	});
	let history_entry = HistoryEntry{sender: from.clone(), timestamp, msg: msg.content};
	match room {
		Some(r) => {
			r.history.push(history_entry);
			draw_messages(state, &r.history);
		},
		None => {
			let new_room = Room{
				id: sodiumoxide::randombytes::randombytes(2).try_into().unwrap(),
				name: "New room".into(),
				members: vec![from],
				history: vec![history_entry],
				unseen: 1,
			};
			account.rooms.push(new_room);
			// draw_rooms(height, sep, &account.rooms);
		}
	}
	write_ron(&account.clone(), "account.ron").expect("couldn't save account");
}

fn sufec_backend<T: FnMut(SufecAddr, u64, Message)>(account: Arc<RwLock<Account>>, receive_msg: T) {
	// Clone the data we need from the account so we don't hold a lock.
	let account_read = account.read().unwrap();
	let sufec_account = account_read.account.clone();
	let prev_eph_sec = account_read.eph_sec.clone();
	drop(account_read);
	// Prepare new key pair.
	let (new_eph_pub, new_eph_sec) = box_::gen_keypair();
	let new_eph_sec_clone = new_eph_sec.clone();
	let save_account = || {
		let mut account = account.write().unwrap();
		account.eph_pub = new_eph_pub;
		account.eph_sec = new_eph_sec_clone;
		write_ron(&account.clone(), "account.ron").expect("couldn't save account");
	};
	// We don't use this but have to pass it in
	let (_, shutdown_rx) = mpsc::channel();
	if let Err(e) = libsufec::listen(
		sufec_account,
		shutdown_rx,
		prev_eph_sec,
		new_eph_pub,
		new_eph_sec,
		save_account,
		receive_msg,
	) {
		eprintln!("error when connecting to homeserver: {}", e)
	}
}
