mod prelude;
use crate::prelude::*;
mod ui;
use crate::ui::*;
mod messaging;
use crate::messaging::*;
mod commands;
use crate::commands::*;
mod util;

const UP_SEQUENCE: &[u8] = &[27, 91, 49, 59, 53, 65];
const DOWN_SEQUENCE: &[u8] = &[27, 91, 49, 59, 53, 66];

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
	draw_ui(&account.read().unwrap(), &state.read().unwrap());
	loop {
		let (nwidth, nheight) = termion::terminal_size().unwrap();
		{
			let mut state = state.write().unwrap();
			if (nwidth, nheight) != (state.width, state.height) {
				state.width = nwidth;
				state.height = nheight;
				draw_ui(&account.read().unwrap(), &state);
			}
		}
		for event in stdin().events() {
			let mut account = account.write().unwrap();
			let mut state = state.write().unwrap();
			handle_input(event.unwrap(), &mut account, &mut state);
		}
	}
}

fn quit() {
	clear();
	std::process::exit(0);
}

fn handle_input(event: Event, account: &mut Account, state: &mut State) {
	match event {
		Event::Key(Key::Esc) => quit(),
		Event::Key(Key::Char(c)) => {
			if c == '\n' {
				submit_message(account, state);
			} else {
				print!("{}", c);
				state.msg_buf.push(c);
			}
			stdout().flush().unwrap();
		},
		Event::Key(Key::Up) => scroll(account, state, 1),
		Event::Key(Key::Down) => scroll(account, state, -1),
		Event::Key(Key::Backspace) => backspace(state),
		Event::Key(Key::Ctrl('n')) => add_room(account, state),
		Event::Key(Key::Ctrl('e')) => {
			if state.msg_buf.is_empty() { return }
			match state.mode {
				Mode::Rooms => rename_room(account, state),
				Mode::Members => rename_member(account, state),
				Mode::Contacts => rename_contact(account, state),
			}
		}
		Event::Key(Key::Ctrl('d')) => {
			match state.mode {
				Mode::Rooms => remove_room(account, state),
				Mode::Members => remove_member(account, state),
				Mode::Contacts => remove_contact(account, state),
			};
		}
		Event::Key(Key::Ctrl('a')) => add_member(account, state),
		Event::Key(Key::Ctrl('p')) => add_contact(account, state),
		Event::Key(Key::Ctrl('r')) => show_rooms(account, state),
		Event::Key(Key::Ctrl('u')) => show_members(account, state),
		Event::Key(Key::Ctrl('c')) => show_contacts(account, state),
		Event::Key(Key::Alt(c)) => {
			if c >= '0' && c <= '9' {
				let n = if c == '0' { 9 } else { c as usize - '0' as usize - 1 };
				let new_room = match account.rooms.get(n) {
					Some(r) => r,
					None => return,
				};
				state.room_id = new_room.id;
				draw_rooms(state, &account.rooms);
				draw_messages(account, state);
				reset_cursor_pos(state);
			}
		}
		Event::Unsupported(seq) => {
			match seq.as_slice() {
				DOWN_SEQUENCE => sidebar_select_relative(account, state, 1),
				UP_SEQUENCE => sidebar_select_relative(account, state, -1),
				_ => {},
			}
		}
		_ => {},
	}
}
