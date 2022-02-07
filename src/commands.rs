use crate::prelude::*;
use crate::ui::*;
use crate::messaging::*;
use crate::util::*;

pub fn clear_input(state: &mut State) {
	state.msg_buf.clear();
	print!("{}{}", clear::CurrentLine, cursor::Goto(1, state.height));
	stdout().flush().unwrap();
}

pub fn backspace(state: &mut State) {
	state.msg_buf.pop();
	print!("{} {}", cursor::Left(1), cursor::Left(1));
	stdout().flush().unwrap();
}

pub fn submit_message(account: &mut Account, state: &mut State) {
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
	reset_cursor_pos(state);
	// Save message history.
	save_account(account).unwrap();
}

pub fn scroll(account: &mut Account, state: &mut State, amount: i16) {
	let pos = match state.scroll.get_mut(&state.room_id) {
		Some(pos) => pos,
		None => return,
	};
	*pos = std::cmp::max(*pos + amount, 0);
	draw_messages(account, state);
	reset_cursor_pos(state);
}

pub fn add_room(account: &mut Account, state: &mut State) {
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
	draw_sidebar(account, state);
	draw_messages(account, state);
	reset_cursor_pos(state);
	save_account(account).unwrap();
}

pub fn rename_room(account: &mut Account, state: &mut State) {
	// Find the room to rename.
	let room = match account.rooms.iter_mut().find(|r| r.id == state.room_id) {
		Some(r) => r,
		None => return,
	};
	room.name = state.msg_buf.clone();
	clear_input(state);
	draw_sidebar(account, state);
	reset_cursor_pos(state);
	save_account(account).unwrap();
}

pub fn remove_room(account: &mut Account, state: &mut State) {
	account.rooms.retain(|r| r.id != state.room_id);
	// Set the current room to another one.
	state.room_id = account.rooms.get(0).map(|r| r.id).unwrap_or_default();
	draw_sidebar(account, state);
	draw_messages(account, state);
	reset_cursor_pos(state);
	save_account(account).unwrap();
}

pub fn add_member(account: &mut Account, state: &mut State) {
	let addr = if let Some(a) = parse_addr_or_name(account, &state.msg_buf) {
		a
	} else { return };
	// Find current room.
	let room = account.rooms.iter_mut().find(|r| r.id == state.room_id);
	if let Some(r) = room {
		r.members.push(addr);
		clear_input(state);
		draw_sidebar(account, state);
		reset_cursor_pos(state);
		save_account(account).unwrap();
	}
}

pub fn remove_member(account: &mut Account, state: &mut State) {
	// Find current room.
	let room = account.rooms.iter_mut().find(|r| r.id == state.room_id);
	if let Some(r) = room {
		r.members.remove(state.selected_index);
		clear_input(state);
		draw_sidebar(account, state);
		reset_cursor_pos(state);
		save_account(account).unwrap();
	}
}

pub fn add_contact(account: &mut Account, state: &mut State) {
	let (addr_raw, name) = match state.msg_buf.split_once(' ') {
		Some(v) => v,
		None => return,
	};
	let addr = match SufecAddr::try_from(addr_raw) {
		Ok(v) => v,
		Err(_) => return,
	};
	// If we already have this address as a contact, update the name.
	if let Some(existing) = account.contacts.iter_mut().find(|c| c.addr == addr) {
		existing.name = name.to_string()
	// Otherwise create a new contact.
	} else {
		account.contacts.push(Contact{addr, name: name.to_string()})
	}
	clear_input(state);
	draw_sidebar(account, state);
	reset_cursor_pos(state);
	save_account(account).unwrap();
}

pub fn rename_member(account: &mut Account, state: &mut State) {
	let room = match account.rooms.iter_mut().find(|r| r.id == state.room_id) {
		Some(r) => r,
		None => return,
	};
	let addr = match room.members.get(state.selected_index) {
		Some(c) => c.clone(),
		None => return,
	};
	// If we already have this address as a contact, update the name.
	if let Some(existing) = account.contacts.iter_mut().find(|c| c.addr == addr) {
		existing.name = state.msg_buf.clone()
	// Otherwise create a new contact.
	} else {
		account.contacts.push(Contact{addr, name: state.msg_buf.clone()})
	}
	clear_input(state);
	draw_sidebar(account, state);
	reset_cursor_pos(state);
	save_account(account).unwrap();
}

pub fn rename_contact(account: &mut Account, state: &mut State) {
	let contact = match account.contacts.get_mut(state.selected_index) {
		Some(c) => c,
		None => return,
	};
	contact.name = state.msg_buf.clone();
	clear_input(state);
	draw_sidebar(account, state);
	reset_cursor_pos(state);
	save_account(account).unwrap();
}

pub fn remove_contact(account: &mut Account, state: &mut State) {
	account.contacts.remove(state.selected_index);
	clear_input(state);
	draw_sidebar(account, state);
	reset_cursor_pos(state);
	save_account(account).unwrap();
}

pub fn show_rooms(account: &mut Account, state: &mut State) {
	state.mode = Mode::Rooms;
	state.selected_index = 0;
	draw_sidebar(account, state);
	reset_cursor_pos(state);
}
pub fn show_members(account: &mut Account, state: &mut State) {
	state.mode = Mode::Members;
	state.selected_index = 0;
	draw_sidebar(account, state);
	reset_cursor_pos(state);
}
pub fn show_contacts(account: &mut Account, state: &mut State) {
	state.mode = Mode::Contacts;
	state.selected_index = 0;
	draw_sidebar(account, state);
	reset_cursor_pos(state);
}
