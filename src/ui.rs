use crate::prelude::*;
use crate::util::*;

pub fn clear() { println!("{}{}", clear::All, cursor::Goto(1, 1)); }

pub fn goto(x: u16, y: u16) { println!("{}", cursor::Goto(x, y)); }

pub fn reset_cursor_pos(state: &State) {
	print!("{}", cursor::Goto(1+state.msg_buf.len() as u16, state.height));
	stdout().flush().unwrap();
}

pub fn draw_ui(account: &Account, state: &State) {
	clear();
	draw_separators(state.height, state.width);
	draw_sidebar(account, state);
	draw_messages(account, state);
	reset_cursor_pos(state);
}

pub fn draw_separators(height: u16, width: u16) {
	// draw buffer separator
	print!("{}{}", cursor::Goto(1, height-1), "-".repeat(width as usize));
	// draw sidebar separator
	let sep = width / 3;
	for y in 1..height-1 {
		print!("{}|", cursor::Goto(sep, y));
	}
}

pub fn draw_sidebar(account: &Account, state: &State) {
	clear_sidebar(state.width, state.height);
	match state.mode {
		Mode::Rooms => draw_rooms(state, &account.rooms),
		Mode::Members => draw_members(account, state),
		Mode::Contacts => draw_contacts(account, state),
	}
}

pub fn clear_sidebar(width: u16, height: u16) {
	let sep = width / 3;
	// Clear any previous room list.
	for y in 1..height - 1 {
		print!("{}{}", cursor::Goto(1, y), " ".repeat((sep - 1) as usize));
	}
}

pub fn draw_rooms(state: &State, rooms: &[Room]) {
	let sep = state.width / 3;
	let mut y = 1;
	for room in rooms {
		// go to position to start room name
		print!("{}|", cursor::Goto(1, y));
		// If it's the current room, highlight it.
		if room.id == state.room_id {
			print!("{}", style::Invert)
		}
		let padding = " ".repeat(sep as usize - 2 - room.name.len());
		print!("{}{}{}", room.name, padding, style::NoInvert);
		// go down and draw a separator line before the next room
		y += 1;
		print!("{}|{}", cursor::Goto(1, y), "-".repeat((sep - 2) as usize));
		y += 1;
	}
	print!("{}", cursor::Goto(1+state.msg_buf.len() as u16, state.height));
}

pub fn draw_members(account: &Account, state: &State) {
	// Find the room.
	let room = match account.rooms.iter().find(|r| r.id == state.room_id) {
		Some(r) => r,
		None => return,
	};
	let sep = state.width / 3;
	let mut y = 1;
	for (i, member) in room.members.iter().enumerate() {
		if i == state.selected_index {
			print!("{}", style::Invert);
		}
		let text = display_addr_or_name(account, member);
		let padding = " ".repeat(sep as usize - 1 - text.len());
		print!("{}{}{}{}", cursor::Goto(1, y), text, padding, style::NoInvert);
		y += 1;
	}
}

pub fn draw_contacts(account: &Account, state: &State) {
	let sep = state.width / 3;
	let mut y = 1;
	for (i, contact) in account.contacts.iter().enumerate() {
		if i == state.selected_index {
			print!("{}", style::Invert);
		}
		let padding = " ".repeat(sep as usize - 1 - contact.name.len());
		print!("{}{}{}{}", cursor::Goto(1, y), contact.name, padding, style::NoInvert);
		y += 1;
	}
}

pub fn draw_messages(account: &Account, state: &State) {
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
		let display_name = display_addr_or_name(account, &message.sender);
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
}
