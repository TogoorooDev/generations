use crate::prelude::*;
use crate::ui::*;

pub fn send_message(account: SufecAccount, recipients: Vec<SufecAddr>, content: MessageContent) {
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

pub fn message_callback(account: &mut Account, state: &mut State, from: SufecAddr, timestamp: u64, msg: Message) {
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
			if state.mode == Mode::Rooms { draw_rooms(state, &account.rooms) }
		}
	}
	reset_cursor_pos(state);
	save_account(account).unwrap();
}

pub fn sufec_backend<T: FnMut(SufecAddr, u64, Message)>(account: Arc<RwLock<Account>>, receive_msg: T) {
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
