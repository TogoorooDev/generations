use crate::prelude::*;
use crate::ui::*;
use crate::messaging::*;
use crate::util::*;
use crate::require_some;

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
	let room = require_some!(account.rooms.iter_mut().find(|r| r.id == state.room_id));
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
	let pos = require_some!(state.scroll.get_mut(&state.room_id));
	*pos = std::cmp::max(*pos + amount, 0);
	draw_messages(account, state);
	reset_cursor_pos(state);
}

pub fn sidebar_select_relative(account: &Account, state: &mut State, change: i8) {
	match state.mode {
		Mode::Rooms => {
			let current = require_some!(account.rooms.iter().position(|r| r.id == state.room_id));
			let intermediate = current as isize + change as isize;
			let new = min(max(intermediate, 0) as usize, account.rooms.len() - 1);
			state.room_id = account.rooms[new].id;
		},
		Mode::Members => {
			let room = require_some!(account.rooms.iter().find(|r| r.id == state.room_id));
			let intermediate = state.selected_index as isize + change as isize;
			state.selected_index = min(max(intermediate, 0) as usize, room.members.len() - 1);
		},
		Mode::Contacts => {
			let intermediate = state.selected_index as isize + change as isize;
			state.selected_index = min(max(intermediate, 0) as usize, account.contacts.len() - 1);
		},
	}
	draw_sidebar(account, state);
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
	let room = require_some!(account.rooms.iter_mut().find(|r| r.id == state.room_id));
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
	let addr = require_some!(parse_addr_or_name(account, &state.msg_buf));
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
		if state.selected_index >= r.members.len() && state.selected_index > 0 {
			state.selected_index -= 1
		}
		clear_input(state);
		draw_sidebar(account, state);
		reset_cursor_pos(state);
		save_account(account).unwrap();
	}
}

pub fn add_contact(account: &mut Account, state: &mut State) {
	let (addr_raw, name) = require_some!(state.msg_buf.split_once(' '));
	let addr = require_some!(SufecAddr::try_from(addr_raw).ok());
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
	let room = require_some!(account.rooms.iter_mut().find(|r| r.id == state.room_id));
	let addr = require_some!(room.members.get(state.selected_index)).clone();
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
	let contact = require_some!(account.contacts.get_mut(state.selected_index));
	contact.name = state.msg_buf.clone();
	clear_input(state);
	draw_sidebar(account, state);
	reset_cursor_pos(state);
	save_account(account).unwrap();
}

pub fn remove_contact(account: &mut Account, state: &mut State) {
	account.contacts.remove(state.selected_index);
	if state.selected_index >= account.contacts.len() && state.selected_index > 0 {
		state.selected_index -= 1
	}
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
