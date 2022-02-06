use anyhow::{Context,Result};
use termion::{cursor, clear, style, input::TermRead, raw::IntoRawMode};
use termion::event::Key;
use libsufec::{Account as SufecAccount, Message, MessageContent, SufecAddr};
use serde::{Deserialize, Serialize};
use sodiumoxide::crypto::box_::{self, PublicKey, SecretKey};
use sodiumoxide::randombytes::randombytes;
use std::io::{stdin, stdout, Write};
use std::fs::File;
use std::collections::HashMap;
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
	id: RoomId,
	name: String,
	members: Vec<SufecAddr>,
	history: Vec<HistoryEntry>,
	unseen: u16,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
struct HistoryEntry {
	sender: SufecAddr,
	timestamp: u64,
	msg: MessageContent,
}
type RoomId = [u8; 2];

struct State {
	room_id: RoomId,
	scroll: HashMap<RoomId, i16>,
	msg_buf: String,
	width: u16,
	height: u16,
}
impl State {
	fn new(account: &Account) -> Self {
		let (width, height) = termion::terminal_size().unwrap();
		let mut scroll = HashMap::new();
		for room in &account.rooms {
			scroll.insert(room.id, 0);
		}
		Self {
			room_id: account.rooms.get(0).map(|r| r.id).unwrap_or_default(),
			msg_buf: String::new(),
			width, height, scroll,
		}
	}
}

fn clear() { println!("{}{}", clear::All, cursor::Goto(1, 1)); }

fn goto(x: u16, y: u16) { println!("{}", cursor::Goto(x, y)); }

fn main() -> Result<()> {
	sodiumoxide::init().expect("sodiumoxie::init failed");
	// Initialize account and state.
	let account = Arc::new(RwLock::new(load_account()?));
	let state = Arc::new(RwLock::new(State::new(&account.read().unwrap())));
	// Start listener.
	let accountclone = Arc::clone(&account);
	let stateclone = Arc::clone(&state);
	let receive_msg = move |from, timestamp, msg| {
		let mut account = accountclone.write().unwrap();
		let mut state = stateclone.write().unwrap();
		message_callback(&mut account, &mut state, from, timestamp, msg);
		save_account(&account).unwrap();
	};
	let accountclone = Arc::clone(&account);
	std::thread::spawn(|| sufec_backend(accountclone, receive_msg));

	let mut _stdout = stdout().into_raw_mode().unwrap();

	// Set up the screen.
	prep(&account.read().unwrap(), &state.read().unwrap());
	
	loop {
		let (nwidth, nheight) = termion::terminal_size().unwrap();
		{
			let mut state = state.write().unwrap();
			if (nwidth, nheight) != (state.width, state.height) {
				state.width = nwidth;
				state.height = nheight;
				prep(&account.read().unwrap(), &state);
			}
		}
		for event in stdin().keys() {
			match event.unwrap() {
			Key::Esc => {
				quit_menu();
			},
			Key::Char(c) => {
				if c == '\n' {
					submit_message(&mut account.write().unwrap(), &mut state.write().unwrap());
				} else {
					print!("{}", c);
					let mut state = state.write().unwrap();
					state.msg_buf.push(c);
				}
				stdout().flush().unwrap();
			},
			Key::Up => scroll(&mut account.write().unwrap(), &mut state.write().unwrap(), 1),
			Key::Down => scroll(&mut account.write().unwrap(), &mut state.write().unwrap(), -1),
			Key::Backspace => backspace(&mut state.write().unwrap()),
			Key::Ctrl('n') => create_room(&mut account.write().unwrap(), &mut state.write().unwrap()),
			Key::Ctrl('a') => add_room_member(&mut account.write().unwrap(), &mut state.write().unwrap()),
			Key::Ctrl('d') => delete_room(&mut account.write().unwrap(), &mut state.write().unwrap()),
			Key::Alt(c) => {
				let account = account.read().unwrap();
				let mut state = state.write().unwrap();
				if c >= '0' && c <= '9' {
					let n = if c == '0' { 9 } else { c as usize - '0' as usize - 1 };
					let new_room = match account.rooms.get(n) {
						Some(r) => r,
						None => continue,
					};
					state.room_id = new_room.id;
					draw_rooms(&state, &account.rooms);
					draw_messages(&account, &state);
				}
			}
			_ => {},
			}
		}
	}
}

fn prep(account: &Account, state: &State) {
	clear();
	draw_bottom(state.height, state.width);
	draw_rooms(state, &account.rooms);
	stdout().flush().unwrap();
	draw_messages(account, state);
}

fn quit_menu(){
	clear();
	std::process::exit(0);
}

fn draw_rooms(state: &State, rooms: &[Room]) {
	let sep = state.width / 3;
	// Clear any previous room list.
	for y in 1..state.height - 1 {
		print!("{}{}", cursor::Goto(1, y), " ".repeat((sep - 2) as usize));
	}
	// draw separator bar
	for y in 1..state.height-1 {
		print!("{}|", cursor::Goto(sep, y));
	}
	let mut y = 1;
	for room in rooms {
		// go to position to start room name
		print!("{}|", cursor::Goto(1, y));
		// If it's the current room, highlight it.
		if room.id == state.room_id {
			print!("{}", style::Invert)
		}
		print!("{}{}{}", room.name, " ".repeat(sep as usize - 2 - room.name.len()), style::NoInvert);
		// go down and draw a separator line before the next room
		y += 1;
		print!("{}|{}", cursor::Goto(1, y), "-".repeat((sep - 2) as usize));
		y += 1;
	}
	print!("{}", cursor::Goto(1+state.msg_buf.len() as u16, state.height));
}

fn draw_bottom(height: u16, width: u16) {
	print!("{}{}", cursor::Goto(1, height-1), "-".repeat(width as usize));
}

fn draw_messages(account: &Account, state: &State) {
	let sep = state.width / 3;
	let message_width = state.width - sep;
	// Clear any previous messages.
	for y in 1..state.height - 1 {
		print!("{}{}", cursor::Goto(sep+1, y), " ".repeat((state.width - sep) as usize));
	}
	// get the current room's history
	let history = match account.rooms.iter().find(|r| r.id == state.room_id) {
		Some(r) => &r.history,
		None => return,
	};
	let scroll = state.scroll[&state.room_id];
	// we don't have automatic GUI-like scrolling, therefore we start from the end
	let mut y = state.height - 2;
	for message in history.iter().rev().skip(scroll as usize) {
		let text = match &message.msg {
			MessageContent::Text(s) => s,
			_ => unimplemented!(),
		};
		let display_name = if message.sender.id == account.account.addr.id {
			"me".to_string()
		} else {
			match account.contacts.iter().find(|c| c.addr.id == message.sender.id) {
				Some(c) => c.name.clone(),
				None => message.sender.clone().to_string(),
			}
		};
		let text = format!("{}: {}", display_name, text);
		let chars: Vec<char> = text.chars().collect();
		let lines = chars.len() as u16 / message_width + 1;
		// Go to the beginning of where the message will span.
		y -= lines - 1;
		goto(sep+1, y);
		let mut index: usize = 0;
		for i in 0..lines {
			let end = std::cmp::min(index+message_width as usize, chars.len());
			let slice = chars[index..end].iter().collect::<String>();
			print!("{}{}", cursor::Goto(sep+1, y+i), slice);
			index += message_width as usize;
		}
		y -= 1;
		if y == 0 { break }
	}
	// Reset the cursor position.
	print!("{}", cursor::Goto(1+state.msg_buf.len() as u16, state.height));
	stdout().flush().unwrap();
}

fn load_account() -> Result<Account> {
	let f = File::open("account.ron").context("couldn't open account.ron")?;
	ron::de::from_reader(f).context("couldn't parse account.ron")
}
fn save_account(account: &Account) -> Result<()> {
	write_ron(account, "account.ron").context("couldn't save account")

}
fn write_ron<T: Serialize>(t: &T, path: &str) -> Result<()> {
	let f = File::create(path).context("couldn't create file")?;
	ron::ser::to_writer_pretty(f, t, ron::ser::PrettyConfig::default())?;
	Ok(())
}

fn submit_message(account: &mut Account, state: &mut State) {
	// Find the room.
	let room = match account.rooms.iter_mut().find(|r| r.id == state.room_id) {
		Some(r) => r,
		None => return,
	};
	// Make the message content.
	let msg = MessageContent::Text(state.msg_buf.clone());
	clear_input(state);
	// Add it to the history.
	let history_entry = HistoryEntry{
		sender: account.account.addr.clone(),
		timestamp: UNIX_EPOCH.elapsed().unwrap().as_micros() as u64,
		msg: msg.clone(),
	};
	room.history.push(history_entry);
	// Send the message.
	send_message(account.account.clone(), room.members.clone(), msg);
	// Update screen.
	draw_messages(account, state);
	// Save message history.
	save_account(account).unwrap();
}

fn send_message(account: SufecAccount, recipients: Vec<SufecAddr>, content: MessageContent) {
	for recipient in recipients.iter() {
		let recipient = recipient.clone();
		let other_recipients = recipients.iter().filter(|r| *r != &recipient).cloned().collect();
		let message = Message{other_recipients, content: content.clone()};
		let account = account.clone();
		std::thread::spawn(move || {
			if let Err(e) = libsufec::send(&account, &recipient, message) {
				eprintln!("couldn't send to {:?}: {}", recipient, e);
			}
		});
	}
}

fn message_callback(account: &mut Account, state: &mut State, from: SufecAddr, timestamp: u64, msg: Message) {
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
			draw_messages(account, state);
		},
		None => {
			let new_room = Room{
				id: randombytes(2).try_into().unwrap(),
				name: "New room".into(),
				members: vec![from],
				history: vec![history_entry],
				unseen: 1,
			};
			state.scroll.insert(new_room.id, 0);
			account.rooms.push(new_room);
			draw_rooms(state, &account.rooms);
		}
	}
	save_account(account).expect("couldn't save account");
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
		save_account(&account).unwrap();
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

fn clear_input(state: &mut State) {
	state.msg_buf.clear();
	print!("{}{}", clear::CurrentLine, cursor::Goto(1, state.height));
	stdout().flush().unwrap();
}

fn backspace(state: &mut State) {
	state.msg_buf.pop();
	print!("{} {}", cursor::Left(1), cursor::Left(1));
	stdout().flush().unwrap();
}

fn scroll(account: &mut Account, state: &mut State, amount: i16) {
	let pos = match state.scroll.get_mut(&state.room_id) {
		Some(pos) => pos,
		None => return,
	};
	*pos = std::cmp::max(*pos + amount, 0);
	draw_messages(account, state);
}

fn create_room(account: &mut Account, state: &mut State) {
	let new_room = Room{
		id: randombytes(2).try_into().unwrap(),
		name: "New room".to_string(),
		members: vec![],
		history: vec![],
		unseen: 0,
	};
	state.scroll.insert(new_room.id, 0);
	state.room_id = new_room.id;
	account.rooms.push(new_room);
	draw_rooms(state, &account.rooms);
	draw_messages(account, state);
	stdout().flush().unwrap();
	save_account(account).unwrap();
}

fn add_room_member(account: &mut Account, state: &mut State) {
	// Parse the address or contact name.
	let addr = if let Some(c) = account.contacts.iter().find(|c| c.name == state.msg_buf) {
		c.addr.clone()
	} else if let Ok(addr) = SufecAddr::try_from(state.msg_buf.as_str()) {
		addr
	} else {
		return
	};
	// Find current room.
	let room = account.rooms.iter_mut().find(|r| r.id == state.room_id);
	if let Some(r) = room {
		if r.members.contains(&addr) { return }
		r.members.push(addr);
		save_account(account).unwrap();
		clear_input(state);
	}
}

fn delete_room(account: &mut Account, state: &mut State) {
	account.rooms.retain(|r| r.id != state.room_id);
	// Set the current room to another one.
	state.room_id = account.rooms.get(0).map(|r| r.id).unwrap_or_default();
	draw_rooms(state, &account.rooms);
	draw_messages(account, state);
	save_account(account).unwrap();
}
