use termion::{cursor, clear, input::TermRead, raw::IntoRawMode};
use termion::event::Key;
use std::io::{stdin, stdout, Write};

fn clear() { println!("{}{}", clear::All, cursor::Goto(1, 1)); }

fn goto(x: u16, y: u16) { println!("{}", cursor::Goto(x, y)); }

fn main() {
	let mut _stdout = stdout().into_raw_mode().unwrap();
	let mut rooms = vec!["Pretty People".to_string(), "Crypto Chat".to_string(), "Free Software Extremists".to_string(), "General".to_string()];

	let (mut width, mut height) = termion::terminal_size().unwrap();
	prep(width, height, &rooms);
	
	loop{

		let (nwidth, nheight) = termion::terminal_size().unwrap();
		if (nwidth, nheight) != (width, height){
			width = nwidth;
			height = nheight;
			prep(width, height, &rooms);
		}

		for event in stdin().keys() {
			match event.unwrap() {
			Key::Esc => {
				quit_menu();
			},
			Key::Char(c) => {
				if c == '\n' {
					print!("{}{}", clear::CurrentLine, cursor::Goto(1, height));
					stdout().flush().unwrap();
				} else {
					print!("{}", c);
					stdout().flush().unwrap();
				}
			},
			_ => {},
			}
		}
	}
}

fn prep(width: u16, height: u16, rooms: &[String]){
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

fn draw_rooms(height: u16, sep: u16, rooms: &[String]) {
	// draw separator bar
	for y in 1..height-1 {
		print!("{}|", cursor::Goto(sep, y));
	}
	let mut y = 1;
	for room in rooms {
		// draw room name
		print!("{}|{}", cursor::Goto(1, y), room);
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
