use anyhow::{Context,Result};
use termion::{cursor, clear, input::TermRead, raw::IntoRawMode};
use termion::event::Key;
use libsufec::{Account as SufecAccount, Message, SufecAddr};
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

type Room = (String, Vec<SufecAddr>);

fn clear() { println!("{}{}", clear::All, cursor::Goto(1, 1)); }

fn goto(x: u16, y: u16) { println!("{}", cursor::Goto(x, y)); }

fn main() -> Result<()> {
	sodiumoxide::init().expect("sodiumoxie::init failed");
	let account = Arc::new(RwLock::new(load_account()?));
	let accountclone = Arc::clone(&account);
	let receive_msg = move |from, msg| {
		let account = accountclone.write().unwrap();
		// message_callback(&from, msg, &account.contacts);
		write_ron(&account.clone(), "account.ron").unwrap();
	};
	let accountclone = Arc::clone(&account);
	std::thread::spawn(|| sufec_backend(accountclone, receive_msg));

	let mut _stdout = stdout().into_raw_mode().unwrap();

	let (mut width, mut height) = termion::terminal_size().unwrap();
	prep(width, height, &account.read().unwrap().rooms);
	
	loop{

		let (nwidth, nheight) = termion::terminal_size().unwrap();
		if (nwidth, nheight) != (width, height){
			width = nwidth;
			height = nheight;
			prep(width, height, &account.read().unwrap().rooms);
		}

		for event in stdin().keys() {
			match event.unwrap() {
			Key::Esc => {
				quit_menu();
			},
			Key::Char(c) => {
				if c == '\n' {
					print!("{}{}", clear::CurrentLine, cursor::Goto(1, height));
				} else {
					print!("{}", c);
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
	draw_messages(height, width, sep, vec!["hi esr nesrot iesrt ertyu sertyu sert yuaeirt eyrt ysert yiueayr iauyer iuayer iuaewr iuwaer ieritseirt aiwetr uae rtayetr yate rywtaeryae ryutersyus eyrituy aer auer tayewrt ayewrt".into(), "hi2".into(), "hi esr nesrot iesrt ertyu sertyu sert yuaeirt eyrt ysert yiueayr iauyer iuayer iuaewr iuwaer ieritseirt aiwetr uae rtayetr yate rywtaeryae ryutersyus eyrituy aer auer tayewrt ayewrt".into()]);
	print!("{}", cursor::Goto(1, height));
	stdout().flush().unwrap();
}

fn quit_menu(){
	clear();
	std::process::exit(0);
	//let (width, height) = termion::terminal_size().unwrap();
}

fn draw_rooms(height: u16, sep: u16, rooms: &[Room]) {
	// draw separator bar
	for y in 1..height-1 {
		print!("{}|", cursor::Goto(sep, y));
	}
	let mut y = 1;
	for room in rooms {
		// draw room name
		print!("{}|{}", cursor::Goto(1, y), room.0);
		// go down and draw a separator line before the next room
		y += 1;
		print!("{}|{}", cursor::Goto(1, y), "-".repeat((sep - 2) as usize));
		y += 1;
	}
}

fn draw_bottom(height: u16, width: u16) {
	print!("{}{}", cursor::Goto(1, height-1), "-".repeat(width as usize));
}

fn draw_messages(height: u16, width: u16, sep: u16, messages: Vec<String>) {
	let message_width = width - sep;
	// we don't have automatic GUI-like scrolling, therefore we start from the end
	let mut y = height - 2;
	for message in messages.iter().rev() {
		let chars: Vec<char> = message.chars().collect();
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
	}
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

fn send_message(account: &SufecAccount, recipients: &[SufecAddr], message: String) {
	let timestamp = UNIX_EPOCH.elapsed().unwrap().as_millis() as u64;
	for recipient in recipients {
		let others = recipients.iter().filter(|r| r != &recipient).map(|r| r.clone()).collect();
		let message = Message::Message(timestamp, others, message.clone());
		if let Err(e) = libsufec::send(account, recipient, message) {
			eprintln!("couldn't send to {:?}: {}", recipient, e);
		}
	}
}

fn sufec_backend<T: FnMut(SufecAddr, Message)>(account: Arc<RwLock<Account>>, receive_msg: T) {
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
