mod prelude;
use crate::prelude::*;
mod ui;
use crate::ui::*;
mod messaging;
use crate::messaging::*;
mod commands;
use crate::commands::*;
mod util;

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
		for event in stdin().keys() {
			let mut account = account.write().unwrap();
			let mut state = state.write().unwrap();
			match event.unwrap() {
			Key::Esc => quit(),
			Key::Char(c) => {
				if c == '\n' {
					submit_message(&mut account, &mut state);
				} else {
					print!("{}", c);
					state.msg_buf.push(c);
				}
				stdout().flush().unwrap();
			},
			Key::Up => scroll(&mut account, &mut state, 1),
			Key::Down => scroll(&mut account, &mut state, -1),
			Key::Backspace => backspace(&mut state),
			Key::Ctrl('n') => add_room(&mut account, &mut state),
			Key::Ctrl('e') => rename_room(&mut account, &mut state),
			Key::Ctrl('d') => {
				match state.mode {
					Mode::Rooms => remove_room(&mut account, &mut state),
					Mode::Members => remove_member(&mut account, &mut state),
					Mode::Contacts => remove_contact(&mut account, &mut state),
				};
			}
			Key::Ctrl('a') => add_member(&mut account, &mut state),
			Key::Ctrl('p') => add_contact(&mut account, &mut state),
			Key::Ctrl('r') => show_rooms(&mut account, &mut state),
			Key::Ctrl('u') => show_members(&mut account, &mut state),
			Key::Ctrl('c') => show_contacts(&mut account, &mut state),
			Key::Alt(c) => {
				if c >= '0' && c <= '9' {
					let n = if c == '0' { 9 } else { c as usize - '0' as usize - 1 };
					let new_room = match account.rooms.get(n) {
						Some(r) => r,
						None => continue,
					};
					state.room_id = new_room.id;
					draw_rooms(&state, &account.rooms);
					draw_messages(&account, &state);
					reset_cursor_pos(&state);
				}
			}
			_ => {},
			}
		}
	}
}

fn quit() {
	clear();
	std::process::exit(0);
}
